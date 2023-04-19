use bstr::ByteSlice;

use crate::base;
use crate::yaml::data::{Data, Id, StringId};
use crate::yaml::error::{Error, ErrorKind};
use crate::yaml::raw::{self, Raw};
use crate::yaml::serde_hint;
use crate::yaml::{Document, Null};

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
        b'\t' | raw::NEWLINE | b'\x0C' | b'\r' | raw::SPACE
    };
}

/// A YAML parser.
#[derive(Clone)]
pub struct Parser<'a> {
    scratch: Vec<u8>,
    data: Data,
    parser: base::Parser<'a>,
}

impl<'a> Parser<'a> {
    /// Construct a new default parser.
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self {
            scratch: Vec::new(),
            data: Data::default(),
            parser: base::Parser::new(input),
        }
    }

    /// Parses a single value, and returns its kind.
    pub(crate) fn parse(mut self) -> Result<Document> {
        let prefix = self.ws();
        let (root, suffix) = self.value(prefix, None, false, true)?;

        let suffix = match suffix {
            Some(suffix) => suffix,
            None => self.ws(),
        };

        if !self.parser.is_eof() {
            return Err(Error::new(self.parser.span(), ErrorKind::ExpectedEof));
        }

        Ok(Document::new(suffix, root, self.data))
    }

    /// Get the current back span.
    fn span_back(&self, string: StringId) -> usize {
        let len = self.data.str(string).len();
        self.parser.pos().saturating_sub(len)
    }

    /// Test if current position contains nothing but whitespace until we reach a line end.
    fn is_eol(&self) -> bool {
        let mut n = self.parser.pos();

        while let Some(&b) = self.parser.get(n) {
            match b {
                raw::NEWLINE => return true,
                b if !b.is_ascii_whitespace() => return false,
                _ => {}
            }

            n = n.wrapping_add(1);
        }

        true
    }

    /// Consume whitespace.
    pub(crate) fn ws_nl(&mut self) -> (StringId, u32) {
        let start = self.parser.pos();
        let mut nl = 0u32;

        loop {
            match self.parser.peek() {
                b'#' => {
                    self.parser.find(raw::NEWLINE);
                }
                ws!() => {}
                _ => break,
            }

            nl = nl.wrapping_add(u32::from(matches!(self.parser.peek(), raw::NEWLINE)));
            self.parser.bump(1);
        }

        (self.data.insert_str(self.parser.string(start)), nl)
    }

    /// Consume whitespace.
    pub(crate) fn ws(&mut self) -> StringId {
        self.ws_nl().0
    }

    /// Consume a single number.
    fn number(&mut self, start: usize, tabular: bool) -> Option<Raw> {
        let mut hint = serde_hint::U64;

        if matches!(self.parser.peek(), b'-') {
            hint = serde_hint::I64;
            self.parser.bump(1);
        }

        let mut dot = false;
        let mut e = false;
        let mut any = false;

        loop {
            match self.parser.peek() {
                b'.' if !dot => {
                    hint = serde_hint::F64;
                    dot = true;
                }
                b'e' | b'E' if !e => {
                    hint = serde_hint::F64;
                    dot = true;
                    e = true;
                }
                b'0'..=b'9' => {}
                _ => {
                    break;
                }
            }

            any = true;
            self.parser.bump(1);
        }

        if !any {
            return None;
        }

        if tabular && !self.is_eol() {
            return None;
        }

        let string = self.data.insert_str(self.parser.string(start));
        Some(Raw::Number(raw::Number::new(string, hint)))
    }

    /// Insert a null value as a placeholder.
    fn placeholder(&mut self, prefix: StringId, parent: Option<Id>) -> Id {
        self.data.insert(Raw::Null(Null::Empty), prefix, parent)
    }

    /// Read a double-quoted string.
    fn single_quoted(&mut self) -> Raw {
        let original = self.parser.pos();
        self.parser.bump(1);
        let start = self.parser.pos();

        loop {
            match self.parser.peek2() {
                (b'\'', b'\'') => {
                    return self.single_quoted_escaped(start, original);
                }
                (b'\'', _) => {
                    break;
                }
                _ => {
                    self.parser.bump(1);
                }
            }
        }

        let string = self.data.insert_str(self.parser.string(start));
        self.parser.bump(usize::from(!self.parser.is_eof()));
        Raw::String(raw::String::new(raw::RawStringKind::Single, string))
    }

    /// Read a single-quoted escaped string.
    fn single_quoted_escaped(&mut self, start: usize, original: usize) -> Raw {
        self.scratch.extend(self.parser.string(start));

        loop {
            match self.parser.peek2() {
                (b'\'', b'\'') => {
                    self.parser.bump(2);
                    self.scratch.push(b'\'');
                }
                (b'\'', _) => {
                    break;
                }
                (b, _) => {
                    self.parser.bump(1);
                    self.scratch.push(b);
                }
            }
        }

        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();
        self.parser.bump(1);

        let original = self.data.insert_str(self.parser.string(original));

        Raw::String(raw::String::new(
            raw::RawStringKind::Original { original },
            string,
        ))
    }

    /// Read a double-quoted string.
    fn double_quoted(&mut self) -> Result<Raw> {
        let original = self.parser.pos();
        self.parser.bump(1);
        let start = self.parser.pos();

        loop {
            match self.parser.peek() {
                b'"' | EOF => break,
                b'\\' => {
                    return self.double_quoted_escaped(start, original);
                }
                _ => {
                    self.parser.bump(1);
                }
            }
        }

        let string = self.data.insert_str(self.parser.string(start));
        self.parser.bump(usize::from(!self.parser.is_eof()));

        Ok(Raw::String(raw::String::new(
            raw::RawStringKind::Double,
            string,
        )))
    }

    /// Parse a double quoted string.
    fn double_quoted_escaped(&mut self, start: usize, original: usize) -> Result<Raw> {
        self.scratch.extend(self.parser.string(start));

        loop {
            match self.parser.peek() {
                b'"' | EOF => break,
                b'\\' => {
                    let start = self.parser.pos();
                    self.parser.bump(1);
                    self.unescape(start)?;
                }
                b => {
                    self.scratch.push(b);
                    self.parser.bump(1);
                }
            }
        }

        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();
        self.parser.bump(1);

        let original = self.data.insert_str(self.parser.string(original));

        Ok(Raw::String(raw::String::new(
            raw::RawStringKind::Original { original },
            string,
        )))
    }

    /// Unescape into the scratch buffer.
    fn unescape(&mut self, start: usize) -> Result<()> {
        let b = match self.parser.peek() {
            b'n' => raw::NEWLINE,
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
                self.parser.bump(1);
                return self.unescape_unicode(start, 2, ErrorKind::BadHexEscape);
            }
            b'u' => {
                self.parser.bump(1);
                return self.unescape_unicode(start, 4, ErrorKind::BadUnicodeEscape);
            }
            _ => {
                self.parser.bump(1);
                return Err(Error::new(start..self.parser.pos(), ErrorKind::BadEscape));
            }
        };

        self.scratch.push(b);
        self.parser.bump(1);
        Ok(())
    }

    /// Unescape a unicode character into the scratch buffer.
    fn unescape_unicode(&mut self, start: usize, count: usize, err: ErrorKind) -> Result<()> {
        let mut c: u32 = 0;

        for _ in 0..count {
            c <<= 4;

            c |= match self.parser.peek() {
                b @ b'0'..=b'9' => u32::from(b - b'0'),
                b @ b'a'..=b'f' => u32::from(b - b'a') + 0xa,
                b @ b'A'..=b'F' => u32::from(b - b'A') + 0xa,
                _ => {
                    self.parser.bump(1);
                    return Err(Error::new(start..self.parser.pos(), err));
                }
            };

            self.parser.bump(1);
        }

        let Some(c) = char::from_u32(c) else {
            return Err(Error::new(start..self.parser.pos(), err));
        };

        self.scratch.extend(c.encode_utf8(&mut [0; 4]).as_bytes());
        Ok(())
    }

    /// Parse an inline sequence.
    fn inline_sequence(&mut self, prefix: StringId, parent: Option<Id>) -> Result<Id> {
        let id = self.placeholder(prefix, parent);

        self.parser.bump(1);

        let mut items = Vec::new();
        let mut last = false;
        let mut trailing = false;
        let mut item_prefix = self.ws();

        while !matches!(self.parser.peek(), b']' | EOF) {
            trailing = false;

            let item_id = self.placeholder(item_prefix, Some(id));
            let value_prefix = self.ws();
            let (value, next_prefix) = self.value(value_prefix, Some(item_id), true, false)?;

            self.data.replace(item_id, raw::SequenceItem { value });

            items.push(item_id);

            if last {
                item_prefix = next_prefix.unwrap_or_else(|| self.ws());
                break;
            }

            if matches!(self.parser.peek(), b',') {
                self.parser.bump(1);
                trailing = true;
            } else {
                last = true;
            }

            item_prefix = next_prefix.unwrap_or_else(|| self.ws());
        }

        if !matches!(self.parser.peek(), b']') {
            return Err(Error::new(
                self.span_back(item_prefix)..self.parser.pos(),
                ErrorKind::BadSequenceTerminator,
            ));
        }

        self.parser.bump(1);

        self.data.replace(
            id,
            raw::Sequence {
                indent: 0,
                kind: raw::SequenceKind::Inline {
                    trailing,
                    suffix: item_prefix,
                },
                items,
            },
        );

        Ok(id)
    }

    /// Parse an inline mapping.
    fn inline_mapping(&mut self, prefix: StringId, parent: Option<Id>) -> Result<Id> {
        let id = self.placeholder(prefix, parent);
        self.parser.bump(1);

        let mut items = Vec::new();
        let mut last = false;
        let mut trailing = false;
        let mut start = self.parser.pos();
        let mut item_prefix = self.ws();

        while !matches!(self.parser.peek(), b'}' | EOF) {
            trailing = false;

            let Some(key) = self.until_colon(self.parser.pos()) else {
                return Err(Error::new(start..self.parser.pos(), ErrorKind::BadMappingSeparator));
            };

            let item_id = self.placeholder(item_prefix, Some(id));
            self.parser.bump(1);
            let value_prefix = self.ws();
            let (value, next_prefix) = self.value(value_prefix, Some(item_id), true, false)?;

            self.data.replace(item_id, raw::MappingItem { key, value });
            items.push(item_id);

            item_prefix = next_prefix.unwrap_or_else(|| self.ws());

            if last {
                break;
            }

            if matches!(self.parser.peek(), b',') {
                self.parser.bump(1);
                trailing = true;
            } else {
                last = true;
            }

            start = self.parser.pos();
            item_prefix = self.ws();
        }

        if !matches!(self.parser.peek(), b'}') {
            return Err(Error::new(
                start..self.parser.pos(),
                ErrorKind::BadMappingTerminator,
            ));
        }

        self.parser.bump(1);

        self.data.replace(
            id,
            raw::Mapping {
                indent: 0,
                items,
                kind: raw::MappingKind::Inline {
                    trailing,
                    suffix: item_prefix,
                },
            },
        );

        Ok(id)
    }

    /// Parse a sequence.
    fn sequence(&mut self, prefix: StringId, parent: Option<Id>) -> Result<(Id, Option<StringId>)> {
        let empty = self.data.insert_str("");
        let mapping_id = self.placeholder(prefix, parent);

        let mut items = Vec::new();
        let mut previous_ws = None;
        let indent = self.indent();

        loop {
            let item_prefix = previous_ws.take().unwrap_or(empty);
            let item_id = self.placeholder(item_prefix, Some(mapping_id));

            self.parser.bump(1);

            let value_prefix = self.ws();
            let (value, ws) = self.value(value_prefix, Some(item_id), false, true)?;

            self.data.replace(item_id, raw::SequenceItem { value });
            items.push(item_id);

            let ws = ws.unwrap_or_else(|| self.ws());
            previous_ws = Some(ws);

            if self.indent() != indent || !matches!(self.parser.peek(), b'-') {
                break;
            }
        }

        self.data.replace(
            mapping_id,
            raw::Sequence {
                indent,
                kind: raw::SequenceKind::Mapping,
                items,
            },
        );

        Ok((mapping_id, previous_ws))
    }

    /// Parse a raw mapping.
    fn mapping(
        &mut self,
        mut start: usize,
        prefix: StringId,
        parent: Option<Id>,
        key: raw::String,
    ) -> Result<(Id, Option<StringId>)> {
        let empty = self.data.insert_str("");
        let mapping_id = self.placeholder(prefix, parent);

        let mut items = Vec::new();
        let mut previous_ws = None;
        let mut current_key = Some(key);

        let indent = self.indent_from(start);

        while let Some(key) = current_key.take() {
            if !matches!(self.parser.peek(), b':') {
                self.parser.bump(1);
                return Err(Error::new(
                    start..self.parser.pos(),
                    ErrorKind::BadMappingSeparator,
                ));
            }

            let item_prefix = previous_ws.take().unwrap_or(empty);
            let item_id = self.placeholder(item_prefix, Some(mapping_id));

            self.parser.bump(1);

            let value_prefix = self.ws();
            let (value, ws) = self.value(value_prefix, Some(item_id), false, true)?;

            self.data.replace(item_id, raw::MappingItem { key, value });
            items.push(item_id);

            let ws = ws.unwrap_or_else(|| self.ws());
            previous_ws = Some(ws);

            if self.indent() != indent {
                break;
            }

            start = self.parser.pos();
            current_key = self.next_mapping_key();
        }

        self.data.replace(
            mapping_id,
            raw::Mapping {
                indent,
                kind: raw::MappingKind::Mapping,
                items,
            },
        );

        Ok((mapping_id, previous_ws))
    }

    /// Count indentation up until the current cursor.
    #[inline]
    fn indent(&self) -> usize {
        self.indent_from(self.parser.pos())
    }

    /// Count indentation up until the current cursor.
    fn indent_from(&self, to: usize) -> usize {
        let string = self.parser.get(..to).unwrap_or_default();
        raw::count_indent(string)
    }

    /// Process a key up until `:` or end of the current line.
    fn key_or_eol(&mut self, start: usize) -> Option<raw::String> {
        self.parser.find2(b':', raw::NEWLINE);

        if self.parser.peek() == b':' {
            let key = self.data.insert_str(self.parser.string(start));
            return Some(raw::String::new(raw::RawStringKind::Bare, key));
        }

        None
    }

    /// Process a key up until `:`.
    fn until_colon(&mut self, start: usize) -> Option<raw::String> {
        while !matches!(self.parser.peek(), b':' | EOF) {
            self.parser.bump(1);
        }

        if self.parser.is_eof() {
            return None;
        }

        let key = self.data.insert_str(self.parser.string(start));
        Some(raw::String::new(raw::RawStringKind::Bare, key))
    }

    /// Process a block as a string.
    fn block(
        &mut self,
        n: usize,
        join: u8,
        folded: bool,
        chomp: bool,
        clip: bool,
    ) -> (Raw, Option<StringId>) {
        let start = self.parser.pos();
        self.parser.bump(n);
        let prefix = self.data.insert_str(self.parser.string(start));

        let start = self.parser.pos();
        let (mut ws, mut nl) = self.ws_nl();

        if nl == 0 {
            self.parser.find(raw::NEWLINE);
            let out = self
                .parser
                .get(start..self.parser.pos())
                .unwrap_or_default()
                .trim();
            self.scratch.extend_from_slice(out);

            (ws, nl) = self.ws_nl();

            if !self.parser.is_eof() {
                self.scratch.push(join);
            }
        }

        let mut end = self.parser.pos();
        let indent = self.indent();

        while !self.parser.is_eof() {
            let s = self.parser.pos();
            self.parser.find(raw::NEWLINE);
            let out = self
                .parser
                .get(s..self.parser.pos())
                .unwrap_or_default()
                .trim();
            self.scratch.extend_from_slice(out);

            end = self.parser.pos();
            (ws, nl) = self.ws_nl();

            if self.indent() < indent {
                break;
            }

            for _ in 0..if folded { 1 } else { nl } {
                self.scratch.push(join);
            }
        }

        for _ in 0..chomp.then_some(nl).unwrap_or_default() {
            self.scratch.push(raw::NEWLINE);

            if clip {
                break;
            }
        }

        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();

        let out = self.parser.get(start..end).unwrap_or_default();
        let original = self.data.insert_str(out);

        let kind = raw::RawStringKind::Multiline { prefix, original };
        (Raw::String(raw::String::new(kind, string)), Some(ws))
    }

    /// Consume a single value.
    fn value(
        &mut self,
        prefix: StringId,
        parent: Option<Id>,
        inline: bool,
        tabular: bool,
    ) -> Result<(Id, Option<StringId>)> {
        let (raw, ws) = match self.parser.peek2() {
            (b'-', ws!()) if !inline => {
                return self.sequence(prefix, parent);
            }
            (b'"', _) => (self.double_quoted()?, None),
            (b'\'', _) => (self.single_quoted(), None),
            (b'[', _) => return Ok((self.inline_sequence(prefix, parent)?, None)),
            (b'{', _) => return Ok((self.inline_mapping(prefix, parent)?, None)),
            (b'~', _) => (Raw::Null(Null::Tilde), None),
            (a @ (b'>' | b'|'), b) => self.block(
                matches!(b, b'-' | b'+').then_some(2).unwrap_or(1),
                if a == b'>' { raw::SPACE } else { raw::NEWLINE },
                a == b'>',
                b != b'-',
                b == b'-',
            ),
            _ => {
                'default: {
                    let start = self.parser.pos();

                    if let Some(number) = self.number(start, tabular) {
                        break 'default (number, None);
                    }

                    if inline {
                        // Seek until we find a control character, since we're
                        // simply treating the current segment as a string.
                        while !matches!(self.parser.peek(), ctl!()) {
                            self.parser.bump(1);
                        }
                    } else if let Some(key) = self.key_or_eol(start) {
                        return self.mapping(start, prefix, parent, key);
                    }

                    // NB: calling `key_or_eol` will have consumed up until end
                    // of line for us, so use the current span as the production
                    // string.
                    match self.parser.string(start) {
                        b"null" => (Raw::Null(Null::Keyword), None),
                        b"true" => (Raw::Boolean(true), None),
                        b"false" => (Raw::Boolean(false), None),
                        string => {
                            let string = self.data.insert_str(string);
                            let string = raw::String::new(raw::RawStringKind::Bare, string);
                            (Raw::String(string), None)
                        }
                    }
                }
            }
        };

        let value = self.data.insert(raw, prefix, parent);
        Ok((value, ws))
    }

    /// Parse next mapping key.
    fn next_mapping_key(&mut self) -> Option<raw::String> {
        let start = self.parser.pos();

        let string = loop {
            match self.parser.peek() {
                b':' | EOF => {
                    let string = self.parser.string(start);

                    if string.is_empty() {
                        return None;
                    }

                    break string;
                }
                _ => {
                    self.parser.bump(1);
                }
            }
        };

        let string = self.data.insert_str(string);
        Some(raw::String::new(raw::RawStringKind::Bare, string))
    }
}
