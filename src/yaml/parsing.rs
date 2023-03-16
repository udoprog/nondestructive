use std::ops::Range;

use bstr::ByteSlice;

use crate::slab::{Pointer, Slab};
use crate::strings::{StringId, Strings};
use crate::yaml::error::{Error, ErrorKind};
use crate::yaml::raw::{Raw, RawListItem, RawNumber, RawString, RawTable, RawTableItem};
use crate::yaml::{Document, StringKind};

use super::raw::RawList;

type Result<T, E = Error> = std::result::Result<T, E>;

const EOF: u8 = b'\0';

#[macro_export]
macro_rules! id_remainder {
    () => {
        b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'0'..=b'9' | b'/' | b'@'
    }
}

macro_rules! number_first {
    () => {
        b'-' | b'0'..=b'9' | b'.'
    };
}

macro_rules! number_remainder {
    () => {
        b'0'..=b'9' | b'.'
    };
}

/// A YAML parser.
#[derive(Clone)]
pub struct Parser<'a> {
    scratch: Vec<u8>,
    strings: Strings,
    tree: Slab<Raw>,
    input: &'a [u8],
    position: usize,
}

impl<'a> Parser<'a> {
    /// Construct a new default parser.
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self {
            scratch: Vec::new(),
            strings: Strings::default(),
            tree: Slab::default(),
            input,
            position: 0,
        }
    }

    /// Peek the next value.
    fn peek(&self) -> u8 {
        let Some(&b) = self.input.get(self.position) else {
            return 0;
        };

        b
    }

    /// Peek the next next value.
    fn peek2(&self) -> (u8, u8) {
        let b0 = self.peek();

        let Some(&b) = self.input.get(self.position.wrapping_add(1)) else {
            return (b0, 0);
        };

        (b0, b)
    }

    /// Insert a value into the tree.
    fn insert(&mut self, raw: Raw) -> Pointer {
        self.tree.insert(raw)
    }

    /// Bump a single byte of input.
    fn bump(&mut self, n: usize) {
        self.position = self.position.wrapping_add(n).min(self.input.len());
    }

    /// Get the current span.
    fn span(&self, len: usize) -> Range<usize> {
        let end = self.position.wrapping_add(len);
        self.position..end
    }

    /// Get a string from the given starting position to current cursor
    /// location.
    fn string(&self, start: usize) -> &'a [u8] {
        self.input.get(start..self.position).unwrap_or_default()
    }

    /// Consume whitespace.
    fn ws(&mut self) -> (StringId, usize) {
        let start = self.position;
        let mut nl = 0;

        while self.peek().is_ascii_whitespace() {
            nl += usize::from(matches!(self.peek(), b'\n'));
            self.bump(1);
        }

        (self.strings.insert(self.string(start)), nl)
    }

    /// Consume a single number.
    fn number(&mut self) -> Result<Raw> {
        let start = self.position;

        if matches!(self.peek(), b'-') {
            self.bump(1);
        }

        while matches!(self.peek(), number_remainder!()) {
            self.bump(1);
        }

        let string = self.strings.insert(self.string(start));
        Ok(Raw::Number(RawNumber::new(string)))
    }

    /// Read a double-quoted string.
    fn single_quoted(&mut self) -> Result<Raw> {
        self.bump(1);
        let start = self.position;

        loop {
            match self.peek2() {
                (b'\'', b'\'') => {
                    self.bump(2);
                }
                (b'\'', _) => {
                    break;
                }
                _ => {
                    self.bump(1);
                }
            }
        }

        let string = self.strings.insert(self.string(start));
        self.bump(1);
        Ok(Raw::String(RawString::new(
            StringKind::SingleQuoted,
            string,
        )))
    }

    /// Read a double-quoted string.
    fn double_quoted(&mut self) -> Result<Raw> {
        self.bump(1);
        let start = self.position;

        while !matches!(self.peek(), b'"' | EOF) {
            self.bump(1);
        }

        let string = self.strings.insert(self.string(start));
        self.bump(1);
        Ok(Raw::String(RawString::new(
            StringKind::DoubleQuoted,
            string,
        )))
    }

    /// Parse a list.
    fn list(&mut self, indentation: StringId) -> Result<(Raw, StringId)> {
        let mut items = Vec::new();
        let mut previous = None;

        let ws = loop {
            if !matches!(self.peek(), b'-') {
                return Err(Error::new(self.span(1), ErrorKind::ExpectedListMarker));
            }

            self.bump(1);
            let (separator, _) = self.ws();
            let new_indentation = self.indentation(&separator);
            let new_indentation = self.build_list_indentation(&indentation, &new_indentation);

            let (value, ws) = self.value(new_indentation, true)?;
            let value = self.tree.insert(value);

            let ws = match ws {
                Some(suffix) => suffix,
                None => self.ws().0,
            };

            items.push(RawListItem {
                prefix: previous.take(),
                separator,
                value,
            });

            let current_indentation = self.indentation(&ws);

            if current_indentation != indentation || !matches!(self.peek(), b'-') {
                break ws;
            }

            previous = Some(ws);
        };

        Ok((Raw::List(RawList { indentation, items }), ws))
    }

    /// Construct list indentation.
    fn build_list_indentation(&mut self, indentation: &StringId, addition: &StringId) -> StringId {
        self.scratch.clear();
        self.scratch
            .extend(self.strings.get(indentation).as_bytes());
        // Account for the list marker (`-`).
        self.scratch.push(b' ');
        self.scratch.extend(self.strings.get(addition).as_bytes());

        let string = self.strings.insert(&self.scratch);
        self.scratch.clear();
        string
    }

    /// Parse a raw table.
    fn table(&mut self, indentation: StringId, mut last_key: RawString) -> Result<(Raw, StringId)> {
        let mut children = Vec::new();
        let mut previous = None;

        let ws = 'outer: loop {
            if !matches!(self.peek(), b':') {
                return Err(Error::new(self.span(1), ErrorKind::ExpectedTableSeparator));
            }

            self.bump(1);

            let (separator, nl) = self.ws();
            let first_indent = self.indentation(&separator);
            let (value, suffix) = self.value(first_indent, nl > 0)?;
            let value = self.tree.insert(value);

            let ws = match suffix {
                Some(suffix) => suffix,
                None => self.ws().0,
            };

            children.push(RawTableItem {
                prefix: previous.take(),
                key: last_key,
                separator,
                value,
            });

            let current_indentation = self.indentation(&ws);

            if current_indentation != indentation {
                break ws;
            }

            let start = self.position;

            previous = Some(ws);
            last_key = loop {
                match self.peek() {
                    b':' | EOF => {
                        let string = self.string(start);

                        if string.is_empty() {
                            break 'outer ws;
                        }

                        let string = self.strings.insert(string);
                        break RawString::new(StringKind::Bare, string);
                    }
                    _ => {
                        self.bump(1);
                    }
                }
            };
        };

        Ok((
            Raw::Table(RawTable {
                indentation,
                items: children,
            }),
            ws,
        ))
    }

    /// Find level of indentation.
    fn indentation(&mut self, string: &StringId) -> StringId {
        let string = self.strings.get(string);
        let indent = string.rfind(b"\n").unwrap_or(0);
        let indent = &string[indent..];
        self.scratch.extend(indent.as_bytes());
        let string = self.strings.insert(&self.scratch);
        self.scratch.clear();
        string
    }

    /// Consume a single value.
    fn value(&mut self, indentation: StringId, table: bool) -> Result<(Raw, Option<StringId>)> {
        let kind = match self.peek2() {
            (b'-', b) if b.is_ascii_whitespace() => {
                let (value, ws) = self.list(indentation)?;
                return Ok((value, Some(ws)));
            }
            (number_first!(), _) => self.number()?,
            (b'"', _) => self.double_quoted()?,
            (b'\'', _) => self.single_quoted()?,
            (b, _) if b.is_ascii_graphic() => {
                let start = self.position;

                loop {
                    match self.peek() {
                        b':' if table => {
                            let string = self.strings.insert(self.string(start));
                            let string = RawString::new(StringKind::Bare, string);
                            let (value, ws) = self.table(indentation, string)?;
                            return Ok((value, Some(ws)));
                        }
                        b'\n' | EOF => {
                            break;
                        }
                        _ => {
                            self.bump(1);
                        }
                    }
                }

                let string = self.strings.insert(self.string(start));
                let string = RawString::new(StringKind::Bare, string);
                Raw::String(string)
            }
            _ => return Err(Error::new(self.span(1), ErrorKind::ValueError)),
        };

        Ok((kind, None))
    }

    /// Parses a single value, and returns its kind.
    pub(crate) fn parse(mut self) -> Result<Document> {
        let (prefix, _) = self.ws();
        let indent = self.indentation(&prefix);
        let (root, suffix) = self.value(indent, true)?;

        let suffix = match suffix {
            Some(suffix) => suffix,
            None => self.ws().0,
        };

        let root = self.insert(root);
        Ok(Document::new(prefix, suffix, root, self.strings, self.tree))
    }
}
