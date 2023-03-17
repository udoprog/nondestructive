use std::ops::Range;

use bstr::ByteSlice;

use crate::strings::{StringId, Strings};
use crate::yaml::error::{Error, ErrorKind};
use crate::yaml::raw::{
    Raw, RawKind, RawListItem, RawListKind, RawNumber, RawString, RawTable, RawTableItem,
    RawTableKind,
};
use crate::yaml::{Document, StringKind};

use super::raw::RawList;

type Result<T, E = Error> = std::result::Result<T, E>;

const EOF: u8 = b'\0';

/// Inline control characters which splits up strings.
macro_rules! inline_control {
    () => {
        b',' | b':' | b']' | b'}' | b'\0'
    };
}

macro_rules! number_first {
    () => {
        b'-' | b'0'..=b'9' | b'.'
    };
}

/// A YAML parser.
#[derive(Clone)]
pub struct Parser<'a> {
    scratch: Vec<u8>,
    strings: Strings,
    input: &'a [u8],
    position: usize,
}

impl<'a> Parser<'a> {
    /// Construct a new default parser.
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self {
            scratch: Vec::new(),
            strings: Strings::default(),
            input,
            position: 0,
        }
    }

    /// Parses a single value, and returns its kind.
    pub(crate) fn parse(mut self) -> Result<Document> {
        let (prefix, _) = self.ws();
        let indent = self.indentation(&prefix);
        let (root, suffix) = self.value(&indent, false)?;

        let suffix = match suffix {
            Some(suffix) => suffix,
            None => self.ws().0,
        };

        Ok(Document::new(prefix, suffix, root, self.strings))
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
    fn number(&mut self) -> Result<RawKind> {
        let start = self.position;

        if matches!(self.peek(), b'-') {
            self.bump(1);
        }

        let mut dot = false;
        let mut e = false;

        loop {
            match self.peek() {
                b'.' if !dot => {
                    dot = true;
                }
                b'e' | b'E' if !e => {
                    dot = true;
                    e = true;
                }
                b'0'..=b'9' => {}
                _ => {
                    break;
                }
            }

            self.bump(1);
        }

        let string = self.strings.insert(self.string(start));
        Ok(RawKind::Number(RawNumber::new(string)))
    }

    /// Read a double-quoted string.
    fn single_quoted(&mut self) -> Result<RawKind> {
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
        Ok(RawKind::String(RawString::new(
            StringKind::SingleQuoted,
            string,
        )))
    }

    /// Read a double-quoted string.
    fn double_quoted(&mut self) -> Result<RawKind> {
        self.bump(1);
        let start = self.position;

        while !matches!(self.peek(), b'"' | EOF) {
            self.bump(1);
        }

        let string = self.strings.insert(self.string(start));
        self.bump(1);
        Ok(RawKind::String(RawString::new(
            StringKind::DoubleQuoted,
            string,
        )))
    }

    /// Parse an inline list.
    fn inline_list(&mut self, indent: &StringId) -> Result<RawKind> {
        self.bump(1);

        let mut items = Vec::new();
        let mut trailing = false;
        let mut prefix = self.ws().0;

        while !matches!(self.peek(), b']' | b'\0') {
            let (value, new_ws) = self.value(indent, true)?;

            let separator = match new_ws {
                Some(ws) => ws,
                None => self.ws().0,
            };

            items.push(RawListItem {
                prefix: Some(prefix),
                separator,
                value: Box::new(value),
            });

            if trailing {
                prefix = self.ws().0;
                break;
            }

            if matches!(self.peek(), b',') {
                self.bump(1);
            } else {
                trailing = true;
            }

            prefix = self.ws().0;
        }

        if !matches!(self.peek(), b']') {
            return Err(Error::new(self.span(1), ErrorKind::ExpectedListTerminator));
        }

        self.bump(1);

        Ok(RawKind::List(RawList {
            kind: RawListKind::Inline {
                trailing: !trailing,
                suffix: prefix,
            },
            items,
        }))
    }

    /// Parse an inline table.
    fn inline_table(&mut self, indent: &StringId) -> Result<RawKind> {
        self.bump(1);

        let mut items = Vec::new();
        let mut last = false;
        let mut trailing = false;
        let mut prefix = self.ws().0;

        while !matches!(self.peek(), b'}' | b'\0') {
            trailing = false;

            let Some(key) = self.key() else {
                return Err(Error::new(self.span(1), ErrorKind::ExpectedTableSeparator));
            };

            self.bump(1);
            let separator = self.ws().0;
            let (value, new_ws) = self.value(indent, true)?;

            items.push(RawTableItem {
                prefix: Some(prefix),
                key,
                separator,
                value: Box::new(value),
            });

            if last {
                prefix = match new_ws {
                    Some(ws) => ws,
                    None => self.ws().0,
                };

                break;
            }

            if matches!(self.peek(), b',') {
                self.bump(1);
                trailing = true;
            } else {
                last = true;
            }

            prefix = match new_ws {
                Some(ws) => ws,
                None => self.ws().0,
            };
        }

        if !matches!(self.peek(), b'}') {
            return Err(Error::new(self.span(1), ErrorKind::ExpectedTableTerminator));
        }

        self.bump(1);

        Ok(RawKind::Table(RawTable {
            kind: RawTableKind::Inline {
                trailing,
                suffix: prefix,
            },
            items,
        }))
    }

    /// Parse a list.
    fn list(&mut self, indentation: &StringId) -> Result<(RawKind, StringId)> {
        let mut items = Vec::new();
        let mut previous = None;
        let indentation_count = self.count_indent(indentation);

        let ws = loop {
            if !matches!(self.peek(), b'-') {
                return Err(Error::new(self.span(1), ErrorKind::ExpectedListMarker));
            }

            self.bump(1);
            let (separator, nl) = self.ws();
            let new_indentation = self.indentation(&separator);

            let new_indent = if nl == 0 {
                self.build_indentation(1, indentation, &new_indentation)
            } else {
                new_indentation
            };

            let (value, ws) = self.value(&new_indent, false)?;

            let ws = match ws {
                Some(suffix) => suffix,
                None => self.ws().0,
            };

            items.push(RawListItem {
                prefix: previous.take(),
                separator,
                value: Box::new(value),
            });

            let current_indentation = self.indentation(&ws);

            if self.count_indent(&current_indentation) != indentation_count {
                break ws;
            }

            if !matches!(self.peek(), b'-') {
                break ws;
            }

            previous = Some(ws);
        };

        Ok((
            RawKind::List(RawList {
                kind: RawListKind::Table,
                items,
            }),
            ws,
        ))
    }

    /// Construct list indentation.
    fn build_indentation(
        &mut self,
        len: usize,
        indentation: &StringId,
        addition: &StringId,
    ) -> StringId {
        self.scratch.clear();
        self.scratch
            .extend(self.strings.get(indentation).as_bytes());
        // Account for any extra spacing that is added, such as the list marker.
        self.scratch.extend(std::iter::repeat(b' ').take(len));
        self.scratch.extend(self.strings.get(addition).as_bytes());

        let string = self.strings.insert(&self.scratch);
        self.scratch.clear();
        string
    }

    /// Parse a raw table.
    fn table(&mut self, indent: &StringId, mut key: RawString) -> Result<(RawKind, StringId)> {
        let mut items = Vec::new();
        let mut previous = None;
        let indent_count = self.count_indent(&indent);

        let ws = loop {
            if !matches!(self.peek(), b':') {
                return Err(Error::new(self.span(1), ErrorKind::ExpectedTableSeparator));
            }

            self.bump(1);
            let (separator, nl) = self.ws();
            let new_indentation = self.indentation(&separator);

            let new_indent = if nl == 0 {
                let len = self.strings.get(&key.string).len();
                self.build_indentation(len.saturating_add(1), &indent, &new_indentation)
            } else {
                new_indentation
            };

            let (value, ws) = self.value(&new_indent, false)?;

            let ws = match ws {
                Some(ws) => ws,
                None => self.ws().0,
            };

            items.push(RawTableItem {
                prefix: previous.take(),
                key,
                separator,
                value: Box::new(value),
            });

            let current_indentation = self.indentation(&ws);

            if self.count_indent(&current_indentation) != indent_count {
                break ws;
            }

            previous = Some(ws);

            key = match self.next_table_key() {
                Some(key) => key,
                None => break ws,
            };
        };

        Ok((
            RawKind::Table(RawTable {
                kind: RawTableKind::Table,
                items,
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

    /// Count indentation level for the given string.
    fn count_indent(&self, string: &StringId) -> usize {
        let string = self.strings.get(string);
        let n = string.rfind(b"\n").map(|n| n.wrapping_add(1)).unwrap_or(0);
        string[n..].chars().count()
    }

    /// Process a key up until `:`.
    fn key(&mut self) -> Option<RawString> {
        let start = self.position;

        loop {
            match self.peek() {
                b':' => {
                    let key = self.strings.insert(self.string(start));
                    return Some(RawString::new(StringKind::Bare, key));
                }
                b'\n' | EOF => {
                    break;
                }
                _ => {
                    self.bump(1);
                }
            }
        }

        None
    }

    /// Consume a single value.
    fn value(&mut self, indent: &StringId, inline: bool) -> Result<(Raw, Option<StringId>)> {
        let (kind, ws) = match self.peek2() {
            (b'-', b) if !inline && b.is_ascii_whitespace() => {
                let (value, ws) = self.list(indent)?;
                (value, Some(ws))
            }
            (number_first!(), _) => (self.number()?, None),
            (b'"', _) => (self.double_quoted()?, None),
            (b'\'', _) => (self.single_quoted()?, None),
            (b'[', _) => (self.inline_list(indent)?, None),
            (b'{', _) => (self.inline_table(indent)?, None),
            (b, _) if b.is_ascii_graphic() => {
                let start = self.position;

                if inline {
                    while !matches!(self.peek(), inline_control!()) {
                        self.bump(1);
                    }
                } else if let Some(key) = self.key() {
                    let (value, ws) = self.table(indent, key)?;
                    return Ok((Raw::new(value, *indent), Some(ws)));
                }

                let string = self.strings.insert(self.string(start));
                let string = RawString::new(StringKind::Bare, string);
                (RawKind::String(string), None)
            }
            _ => return Err(Error::new(self.span(1), ErrorKind::ValueError)),
        };

        Ok((Raw::new(kind, *indent), ws))
    }

    /// Parse next table key.
    fn next_table_key(&mut self) -> Option<RawString> {
        let start = self.position;

        let string = loop {
            match self.peek() {
                b':' | EOF => {
                    let string = self.string(start);

                    if string.is_empty() {
                        return None;
                    }

                    break string;
                }
                _ => {
                    self.bump(1);
                }
            }
        };

        let string = self.strings.insert(string);
        Some(RawString::new(StringKind::Bare, string))
    }
}
