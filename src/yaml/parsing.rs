use std::ops::Range;

use bstr::ByteSlice;

use crate::slab::{Pointer, Slab};
use crate::strings::{StringId, Strings};
use crate::yaml::error::{Error, ErrorKind};
use crate::yaml::raw::{Raw, RawNumber, RawString};
use crate::yaml::{Document, StringKind};

use super::raw::{RawTable, RawTableElement};

type Result<T, E = Error> = std::result::Result<T, E>;

macro_rules! id_first {
    () => {
        b'a'..=b'z' | b'A'..=b'Z' | b'@'
    }
}

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
    fn ws(&mut self) -> StringId {
        let start = self.position;

        while self.peek().is_ascii_whitespace() {
            self.bump(1);
        }

        self.strings.insert(self.string(start))
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

    /// Read a string from a line.
    fn eol(&mut self, start: usize) -> Result<Raw> {
        while !matches!(self.peek(), b'\n' | b'\0') {
            self.bump(1);
        }

        let string = self.strings.insert(self.string(start));
        Ok(Raw::String(RawString::new(StringKind::Bare, string)))
    }

    /// Try to read an identifier.
    fn id(&mut self) -> Option<usize> {
        if !matches!(self.peek(), id_first!()) {
            return None;
        }

        Some(self.id_remainder())
    }

    /// Read the remainder of an identifier.
    fn id_remainder(&mut self) -> usize {
        let start = self.position;

        self.bump(1);

        while matches!(self.peek(), id_remainder!()) {
            self.bump(1);
        }

        start
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

        while !matches!(self.peek(), b'"' | b'\0') {
            self.bump(1);
        }

        let string = self.strings.insert(self.string(start));
        self.bump(1);
        Ok(Raw::String(RawString::new(
            StringKind::DoubleQuoted,
            string,
        )))
    }

    /// Parse a raw table.
    fn table(
        &mut self,
        indent: StringId,
        mut last_key: RawString,
    ) -> Result<(Raw, Option<StringId>)> {
        let mut children = Vec::new();
        let mut last_suffix = None;

        let current_prefix = loop {
            if !matches!(self.peek(), b':') {
                return Err(Error::new(self.span(1), ErrorKind::ExpectedTableSeparator));
            }

            self.bump(1);

            let separator = self.ws();
            let first_indent = self.indent(&separator);
            let (first_value, first_suffix) = self.value(first_indent)?;
            let first_value = self.tree.insert(first_value);

            let suffix = match first_suffix {
                Some(suffix) => suffix,
                None => self.ws(),
            };

            children.push(RawTableElement {
                prefix: last_suffix.take(),
                key: last_key,
                separator,
                value: first_value,
            });

            let Some(current_indent) = self.indent(&suffix) else {
                break Some(suffix);
            };

            if current_indent != indent || !matches!(self.peek(), id_first!()) {
                break Some(suffix);
            }

            last_suffix = Some(suffix);

            last_key = match self.id() {
                Some(last) => {
                    let string = self.string(last);
                    let string = self.strings.insert(string);
                    RawString::new(StringKind::Bare, string)
                }
                None => {
                    break Some(suffix);
                }
            };
        };

        Ok((Raw::Table(RawTable { indent, children }), current_prefix))
    }

    fn indent(&mut self, string: &StringId) -> Option<StringId> {
        {
            let string = self.strings.get(string);
            let indent = string.rfind(b"\n")?;
            let indent = string.get(indent..)?;
            self.scratch.clear();
            self.scratch.extend(indent);
        }

        Some(self.strings.insert(&self.scratch))
    }

    /// Consume a single value.
    fn value(&mut self, indent: Option<StringId>) -> Result<(Raw, Option<StringId>)> {
        let kind = match self.peek() {
            number_first!() => self.number()?,
            b'"' => self.double_quoted()?,
            b'\'' => self.single_quoted()?,
            id_first!() => {
                let start = self.id_remainder();

                let (Some(indent), b':') = (indent, self.peek()) else {
                    return Ok((self.eol(start)?, None));
                };

                let string = self.strings.insert(self.string(start));
                let string = RawString::new(StringKind::Bare, string);
                return self.table(indent, string);
            }
            _ => return Err(Error::new(self.span(1), ErrorKind::ValueError)),
        };

        Ok((kind, None))
    }

    /// Parses a single value, and returns its kind.
    pub(crate) fn parse(mut self) -> Result<Document> {
        let prefix = self.ws();
        let indent = self.indent(&prefix);
        let (root, suffix) = self.value(indent)?;

        let suffix = match suffix {
            Some(suffix) => suffix,
            None => self.ws(),
        };

        let root = self.insert(root);
        Ok(Document::new(prefix, suffix, root, self.strings, self.tree))
    }
}
