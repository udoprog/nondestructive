use bstr::ByteSlice;

use crate::yaml::data::{Data, StringId, ValueId};
use crate::yaml::error::{Error, ErrorKind};
use crate::yaml::raw::{
    Raw, RawMapping, RawMappingItem, RawMappingKind, RawNumber, RawSequence, RawSequenceItem,
    RawSequenceKind, RawString, RawStringKind, NEWLINE, SPACE,
};
use crate::yaml::serde;
use crate::yaml::{Document, NullKind};

type Result<T, E = Error> = std::result::Result<T, E>;

const EOF: u8 = b'\0';

/// Inline control characters which splits up data.
macro_rules! ctl {
    () => {
        b',' | b':' | b']' | b'}' | EOF
    };
}

/// Ascii whitespace matching.
macro_rules! ws {
    () => {
        b'\t' | NEWLINE | b'\x0C' | b'\r' | SPACE
    };
}

/// A YAML parser.
#[derive(Clone)]
pub struct Parser<'a> {
    scratch: Vec<u8>,
    data: Data,
    input: &'a [u8],
    n: usize,
}

impl<'a> Parser<'a> {
    /// Construct a new default parser.
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self {
            scratch: Vec::new(),
            data: Data::default(),
            input,
            n: 0,
        }
    }

    /// Parses a single value, and returns its kind.
    pub(crate) fn parse(mut self) -> Result<Document> {
        let prefix = self.ws();
        let indent = self.indentation(&prefix);
        let (root, suffix) = self.value(&indent, None, false)?;

        let suffix = match suffix {
            Some(suffix) => suffix,
            None => self.ws(),
        };

        Ok(Document::new(prefix, suffix, root, self.data))
    }

    /// Test if eof.
    fn is_eof(&self) -> bool {
        self.n == self.input.len()
    }

    /// Peek the next value.
    fn peek(&self) -> u8 {
        let Some(&b) = self.input.get(self.n) else {
            return 0;
        };

        b
    }

    /// Peek the next next value.
    fn peek2(&self) -> (u8, u8) {
        let b0 = self.peek();

        let Some(&b) = self.input.get(self.n.wrapping_add(1)) else {
            return (b0, 0);
        };

        (b0, b)
    }

    /// Bump a single byte of input.
    fn bump(&mut self, n: usize) {
        self.n = self.n.wrapping_add(n).min(self.input.len());
    }

    /// Get the current back span.
    fn span_back(&self, string: &StringId) -> usize {
        let len = self.data.str(string).len();
        self.n.saturating_sub(len)
    }

    /// Get a string from the given starting position to current cursor
    /// location.
    fn string(&self, start: usize) -> &'a [u8] {
        self.input.get(start..self.n).unwrap_or_default()
    }

    /// Find the given character.
    fn find(&mut self, a: u8) {
        let input = self.input.get(self.n..).unwrap_or_default();

        if let Some(n) = memchr::memchr(a, input) {
            self.bump(n);
        } else {
            self.n = self.input.len();
        }
    }

    /// Find the given character.
    fn find2(&mut self, a: u8, b: u8) {
        let input = self.input.get(self.n..).unwrap_or_default();

        if let Some(n) = memchr::memchr2(a, b, input) {
            self.bump(n);
        } else {
            self.n = self.input.len();
        }
    }

    /// Consume whitespace.
    fn ws_nl(&mut self) -> (StringId, u32) {
        let start = self.n;
        let mut nl = 0u32;

        loop {
            match self.peek() {
                b'#' => {
                    self.find(NEWLINE);
                }
                ws!() => {}
                _ => break,
            }

            nl = nl.wrapping_add(u32::from(matches!(self.peek(), NEWLINE)));
            self.bump(1);
        }

        (self.data.insert_str(self.string(start)), nl)
    }

    /// Consume whitespace.
    fn ws(&mut self) -> StringId {
        self.ws_nl().0
    }

    /// Consume a single number.
    fn number(&mut self, start: usize) -> Option<Raw> {
        let mut hint = serde::U64;

        if matches!(self.peek(), b'-') {
            hint = serde::I64;
            self.bump(1);
        }

        let mut dot = false;
        let mut e = false;
        let mut any = false;

        loop {
            match self.peek() {
                b'.' if !dot => {
                    hint = serde::F64;
                    dot = true;
                }
                b'e' | b'E' if !e => {
                    hint = serde::F64;
                    dot = true;
                    e = true;
                }
                b'0'..=b'9' => {}
                _ => {
                    break;
                }
            }

            any = true;
            self.bump(1);
        }

        if !any {
            return None;
        }

        let string = self.data.insert_str(self.string(start));
        Some(Raw::Number(RawNumber::new(string, hint)))
    }

    /// Insert a null value as a placeholder.
    fn insert_null(&mut self, indent: &StringId, parent: Option<ValueId>) -> ValueId {
        self.data
            .insert(Raw::Null(NullKind::Empty), *indent, parent)
    }

    /// Read a double-quoted string.
    fn single_quoted(&mut self) -> Raw {
        let original = self.n;
        self.bump(1);
        let start = self.n;

        loop {
            match self.peek2() {
                (b'\'', b'\'') => {
                    return self.single_quoted_escaped(start, original);
                }
                (b'\'', _) => {
                    break;
                }
                _ => {
                    self.bump(1);
                }
            }
        }

        let string = self.data.insert_str(self.string(start));
        self.bump(usize::from(!self.is_eof()));
        Raw::String(RawString::new(RawStringKind::SingleQuoted, string))
    }

    /// Read a single-quoted escaped string.
    fn single_quoted_escaped(&mut self, start: usize, original: usize) -> Raw {
        self.scratch.extend(self.string(start));

        loop {
            match self.peek2() {
                (b'\'', b'\'') => {
                    self.bump(2);
                    self.scratch.push(b'\'');
                }
                (b'\'', _) => {
                    break;
                }
                (b, _) => {
                    self.bump(1);
                    self.scratch.push(b);
                }
            }
        }

        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();
        self.bump(1);

        let original = self.data.insert_str(self.string(original));

        Raw::String(RawString::new(RawStringKind::Original(original), string))
    }

    /// Read a double-quoted string.
    fn double_quoted(&mut self) -> Result<Raw> {
        let original = self.n;
        self.bump(1);
        let start = self.n;

        loop {
            match self.peek() {
                b'"' | EOF => break,
                b'\\' => {
                    return self.double_quoted_escaped(start, original);
                }
                _ => {
                    self.bump(1);
                }
            }
        }

        let string = self.data.insert_str(self.string(start));
        self.bump(usize::from(!self.is_eof()));

        Ok(Raw::String(RawString::new(
            RawStringKind::DoubleQuoted,
            string,
        )))
    }

    /// Parse a double quoted string.
    fn double_quoted_escaped(&mut self, start: usize, original: usize) -> Result<Raw> {
        self.scratch.extend(self.string(start));

        loop {
            match self.peek() {
                b'"' | EOF => break,
                b'\\' => {
                    let start = self.n;
                    self.bump(1);
                    self.unescape(start)?;
                }
                b => {
                    self.scratch.push(b);
                    self.bump(1);
                }
            }
        }

        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();
        self.bump(1);

        let original = self.data.insert_str(self.string(original));

        Ok(Raw::String(RawString::new(
            RawStringKind::Original(original),
            string,
        )))
    }

    /// Unescape into the scratch buffer.
    fn unescape(&mut self, start: usize) -> Result<()> {
        let b = match self.peek() {
            b'n' => NEWLINE,
            b'0' => b'\x00',
            b'a' => b'\x07',
            b'b' => b'\x08',
            b't' => b'\x09',
            b'v' => b'\x0b',
            b'f' => b'\x0c',
            b'r' => b'\r',
            b'e' => b'\x1b',
            b'\\' => b'\"',
            b'x' => {
                self.bump(1);
                return self.unescape_unicode(start, 2, ErrorKind::BadHexEscape);
            }
            b'u' => {
                self.bump(1);
                return self.unescape_unicode(start, 4, ErrorKind::BadUnicodeEscape);
            }
            _ => {
                self.bump(1);
                return Err(Error::new(start..self.n, ErrorKind::BadEscape));
            }
        };

        self.scratch.push(b);
        self.bump(1);
        Ok(())
    }

    /// Unescape a unicode character into the scratch buffer.
    fn unescape_unicode(&mut self, start: usize, count: usize, err: ErrorKind) -> Result<()> {
        let mut c: u32 = 0;

        for _ in 0..count {
            c <<= 4;

            c |= match self.peek() {
                b @ b'0'..=b'9' => u32::from(b - b'0'),
                b @ b'a'..=b'f' => u32::from(b - b'a') + 0xa,
                b @ b'A'..=b'F' => u32::from(b - b'A') + 0xa,
                _ => {
                    self.bump(1);
                    return Err(Error::new(start..self.n, err));
                }
            };

            self.bump(1);
        }

        let Some(c) = char::from_u32(c) else {
            return Err(Error::new(start..self.n, err));
        };

        self.scratch.extend(c.encode_utf8(&mut [0; 4]).as_bytes());
        Ok(())
    }

    /// Parse an inline sequence.
    fn inline_sequence(&mut self, indent: &StringId, parent: Option<ValueId>) -> Result<ValueId> {
        let id = self.insert_null(indent, parent);

        self.bump(1);

        let mut items = Vec::new();
        let mut last = false;
        let mut trailing = false;
        let mut prefix = self.ws();

        while !matches!(self.peek(), b']' | EOF) {
            trailing = false;
            let (value, new_ws) = self.value(indent, Some(id), true)?;

            let separator = match new_ws {
                Some(ws) => ws,
                None => self.ws(),
            };

            items.push(RawSequenceItem {
                prefix: Some(prefix),
                separator,
                value,
            });

            if last {
                prefix = self.ws();
                break;
            }

            if matches!(self.peek(), b',') {
                self.bump(1);
                trailing = true;
            } else {
                last = true;
            }

            prefix = self.ws();
        }

        if !matches!(self.peek(), b']') {
            return Err(Error::new(
                self.span_back(&prefix)..self.n,
                ErrorKind::BadSequenceTerminator,
            ));
        }

        self.bump(1);

        self.data.replace(
            id,
            Raw::Sequence(RawSequence {
                kind: RawSequenceKind::Inline {
                    trailing,
                    suffix: prefix,
                },
                items,
            }),
        );

        Ok(id)
    }

    /// Parse an inline mapping.
    fn inline_mapping(&mut self, indent: &StringId, parent: Option<ValueId>) -> Result<ValueId> {
        let id = self.insert_null(indent, parent);

        self.bump(1);

        let mut items = Vec::new();
        let mut last = false;
        let mut trailing = false;
        let mut start = self.n;
        let mut prefix = self.ws();

        while !matches!(self.peek(), b'}' | EOF) {
            trailing = false;

            let Some(key) = self.until_colon(self.n) else {
                return Err(Error::new(start..self.n, ErrorKind::BadMappingSeparator));
            };

            self.bump(1);
            let separator = self.ws();
            let (value, new_ws) = self.value(indent, Some(id), true)?;

            items.push(RawMappingItem {
                prefix: Some(prefix),
                key,
                separator,
                value,
            });

            if last {
                prefix = match new_ws {
                    Some(ws) => ws,
                    None => self.ws(),
                };

                break;
            }

            if matches!(self.peek(), b',') {
                self.bump(1);
                trailing = true;
            } else {
                last = true;
            }

            start = self.n;

            prefix = match new_ws {
                Some(ws) => ws,
                None => self.ws(),
            };
        }

        if !matches!(self.peek(), b'}') {
            return Err(Error::new(start..self.n, ErrorKind::BadMappingTerminator));
        }

        self.bump(1);

        self.data.replace(
            id,
            Raw::Mapping(RawMapping {
                items,
                kind: RawMappingKind::Inline {
                    trailing,
                    suffix: prefix,
                },
            }),
        );

        Ok(id)
    }

    /// Parse a sequence.
    fn sequence(
        &mut self,
        indent: &StringId,
        parent: Option<ValueId>,
    ) -> Result<(ValueId, StringId)> {
        let id = self.insert_null(indent, parent);

        let mut items = Vec::new();
        let mut previous = None;
        let indentation_count = self.count_indent(indent);

        let ws = loop {
            self.bump(1);
            let (separator, nl) = self.ws_nl();
            let new_indentation = self.indentation(&separator);

            let new_indent = if nl == 0 {
                self.build_indentation(1, indent, &new_indentation)
            } else {
                new_indentation
            };

            let (value, ws) = self.value(&new_indent, Some(id), false)?;

            let ws = match ws {
                Some(suffix) => suffix,
                None => self.ws(),
            };

            items.push(RawSequenceItem {
                prefix: previous.take(),
                separator,
                value,
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

        self.data.replace(
            id,
            Raw::Sequence(RawSequence {
                kind: RawSequenceKind::Mapping,
                items,
            }),
        );

        Ok((id, ws))
    }

    /// Construct sequence indentation.
    fn build_indentation(
        &mut self,
        len: usize,
        indentation: &StringId,
        addition: &StringId,
    ) -> StringId {
        self.scratch.clear();
        self.scratch.extend(self.data.str(indentation).as_bytes());
        // Account for any extra spacing that is added, such as the sequence marker.
        self.scratch.extend(std::iter::repeat(SPACE).take(len));
        self.scratch.extend(self.data.str(addition).as_bytes());

        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();
        string
    }

    /// Parse a raw mapping.
    fn mapping(
        &mut self,
        mut start: usize,
        indent: &StringId,
        parent: Option<ValueId>,
        mut key: RawString,
    ) -> Result<(ValueId, StringId)> {
        let id = self.insert_null(indent, parent);

        let mut items = Vec::new();
        let mut previous = None;
        let indent_count = self.count_indent(indent);

        let ws = loop {
            if !matches!(self.peek(), b':') {
                self.bump(1);
                return Err(Error::new(start..self.n, ErrorKind::BadMappingSeparator));
            }

            self.bump(1);
            let (separator, nl) = self.ws_nl();
            let new_indentation = self.indentation(&separator);

            let new_indent = if nl == 0 {
                let len = self.data.str(&key.string).len();
                self.build_indentation(len.saturating_add(1), indent, &new_indentation)
            } else {
                new_indentation
            };

            let (value, ws) = self.value(&new_indent, Some(id), false)?;

            let ws = match ws {
                Some(ws) => ws,
                None => self.ws(),
            };

            items.push(RawMappingItem {
                prefix: previous.take(),
                key,
                separator,
                value,
            });

            let current_indentation = self.indentation(&ws);

            if self.count_indent(&current_indentation) != indent_count {
                break ws;
            }

            previous = Some(ws);
            start = self.n;

            key = match self.next_mapping_key() {
                Some(key) => key,
                None => break ws,
            };
        };

        self.data.replace(
            id,
            Raw::Mapping(RawMapping {
                kind: RawMappingKind::Mapping,
                items,
            }),
        );

        Ok((id, ws))
    }

    /// Find level of indentation.
    fn indentation(&mut self, string: &StringId) -> StringId {
        let string = self.data.str(string);
        let indent = string.rfind(b"\n").unwrap_or(0);
        let indent = &string[indent..];
        self.scratch.extend(indent.as_bytes());
        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();
        string
    }

    /// Count indentation level for the given string.
    fn count_indent(&self, string: &StringId) -> usize {
        let string = self.data.str(string);
        let n = string.rfind(b"\n").map_or(0, |n| n.wrapping_add(1));
        string[n..].chars().count()
    }

    /// Process a key up until `:` or end of the current line.
    fn key_or_eol(&mut self, start: usize) -> Option<RawString> {
        self.find2(b':', NEWLINE);

        if self.peek() == b':' {
            let key = self.data.insert_str(self.string(start));
            return Some(RawString::new(RawStringKind::Bare, key));
        }

        None
    }

    /// Process a key up until `:`.
    fn until_colon(&mut self, start: usize) -> Option<RawString> {
        while !matches!(self.peek(), b':' | EOF) {
            self.bump(1);
        }

        if self.is_eof() {
            return None;
        }

        let key = self.data.insert_str(self.string(start));
        Some(RawString::new(RawStringKind::Bare, key))
    }

    /// Process a multiline string.
    fn multiline_string(&mut self, join: u8) -> (Raw, Option<StringId>) {
        let prefix = self.peek();
        self.bump(1);

        let start = self.n;
        let (mut ws, nl) = self.ws_nl();

        if nl == 0 {
            self.find(NEWLINE);
            let out = self.input.get(start..self.n).unwrap_or_default().trim();
            self.scratch.extend_from_slice(out);

            ws = self.ws();

            if !self.is_eof() {
                self.scratch.push(join);
            }
        }

        let mut end = self.n;
        let indent = self.count_indent(&ws);

        while !self.is_eof() {
            let s = self.n;
            self.find(NEWLINE);
            let out = self.input.get(s..self.n).unwrap_or_default().trim();
            self.scratch.extend_from_slice(out);

            end = self.n;
            ws = self.ws();

            if self.count_indent(&ws) < indent {
                break;
            }

            self.scratch.push(join);
        }

        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();

        let out = self.input.get(start..end).unwrap_or_default();
        let original = self.data.insert_str(out);

        let kind = RawStringKind::Multiline(prefix, original);
        (Raw::String(RawString::new(kind, string)), Some(ws))
    }

    /// Consume a single value.
    fn value(
        &mut self,
        indent: &StringId,
        parent: Option<ValueId>,
        inline: bool,
    ) -> Result<(ValueId, Option<StringId>)> {
        let (kind, ws) = match self.peek2() {
            (b'-', ws!()) if !inline => {
                let (value, ws) = self.sequence(indent, parent)?;
                return Ok((value, Some(ws)));
            }
            (b'"', _) => (self.double_quoted()?, None),
            (b'\'', _) => (self.single_quoted(), None),
            (b'[', _) => return Ok((self.inline_sequence(indent, parent)?, None)),
            (b'{', _) => return Ok((self.inline_mapping(indent, parent)?, None)),
            (b'~', _) => (Raw::Null(NullKind::Tilde), None),
            (b'|', _) => self.multiline_string(NEWLINE),
            (b'>', _) => self.multiline_string(SPACE),
            _ => {
                'default: {
                    let start = self.n;

                    if let Some(number) = self.number(start) {
                        break 'default (number, None);
                    }

                    if inline {
                        // Seek until we find a control character, since we're
                        // simply treating the current segment as a string.
                        while !matches!(self.peek(), ctl!()) {
                            self.bump(1);
                        }
                    } else if let Some(key) = self.key_or_eol(start) {
                        let (value, ws) = self.mapping(start, indent, parent, key)?;
                        return Ok((value, Some(ws)));
                    }

                    // NB: calling `key_or_eol` will have consumed up until end
                    // of line for us, so use the current span as the production
                    // string.
                    match self.string(start) {
                        b"null" => (Raw::Null(NullKind::Keyword), None),
                        b"true" => (Raw::Boolean(true), None),
                        b"false" => (Raw::Boolean(false), None),
                        string => {
                            let string = self.data.insert_str(string);
                            let string = RawString::new(RawStringKind::Bare, string);
                            (Raw::String(string), None)
                        }
                    }
                }
            }
        };

        let value = self.data.insert(kind, *indent, parent);
        Ok((value, ws))
    }

    /// Parse next mapping key.
    fn next_mapping_key(&mut self) -> Option<RawString> {
        let start = self.n;

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

        let string = self.data.insert_str(string);
        Some(RawString::new(RawStringKind::Bare, string))
    }
}
