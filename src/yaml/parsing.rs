use std::array;

use bstr::ByteSlice;

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
        let prefix = self.start_of_document();

        let (root, suffix) = self.value(prefix, None, false, true, None)?;

        let suffix = match suffix {
            Some(suffix) => suffix,
            None => self.ws(),
        };

        if !self.is_eof() {
            return Err(Error::new(self.n..self.input.len(), ErrorKind::ExpectedEof));
        }

        Ok(Document::new(suffix, root, self.data))
    }

    /// Process document delimiter.
    ///
    /// This is a `---` that is allowed to exist at the beginning of the document.
    fn start_of_document(&mut self) -> StringId {
        let mut prefix = self.ws();

        loop {
            match self.peek() {
                // Process headers.
                [b'%', _, _] => {
                    self.find(raw::NEWLINE);
                    prefix = self.ws();
                }
                // Process start-of-document.
                [b'-', b'-', b'-'] => {
                    self.bump(3);
                    prefix = self.ws();
                    break;
                }
                _ => {
                    break;
                }
            }
        }

        prefix
    }

    /// Test if eof.
    fn is_eof(&self) -> bool {
        self.n == self.input.len()
    }

    /// Peek the next value.
    fn peek1(&self) -> u8 {
        let [a] = self.peek();
        a
    }

    /// Peek the next next value.
    fn peek2(&self) -> (u8, u8) {
        let [a, b] = self.peek();
        (a, b)
    }

    /// Peek the next three values.
    fn peek<const N: usize>(&self) -> [u8; N] {
        array::from_fn(|n| {
            self.input
                .get(self.n.wrapping_add(n))
                .copied()
                .unwrap_or(EOF)
        })
    }

    /// Bump a single byte of input.
    fn bump(&mut self, n: usize) {
        self.n = self.n.wrapping_add(n).min(self.input.len());
    }

    /// Get the current back span.
    fn span_back(&self, string: StringId) -> usize {
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
            match self.peek1() {
                b'#' => {
                    self.find(raw::NEWLINE);
                }
                ws!() => {}
                _ => break,
            }

            nl = nl.wrapping_add(u32::from(matches!(self.peek1(), raw::NEWLINE)));
            self.bump(1);
        }

        (self.data.insert_str(self.string(start)), nl)
    }

    /// Consume whitespace.
    fn ws(&mut self) -> StringId {
        self.ws_nl().0
    }

    /// Test if current position contains nothing but whitespace until we reach a line end.
    fn is_eol(&self) -> bool {
        let mut n = self.n;

        while let Some(&b) = self.input.get(n) {
            match b {
                raw::NEWLINE => return true,
                b if !b.is_ascii_whitespace() => return false,
                _ => {}
            }

            n = n.wrapping_add(1);
        }

        true
    }

    /// Consume a single number.
    fn number(&mut self, start: usize, tabular: bool) -> Option<Raw> {
        let mut hint = serde_hint::U64;

        if matches!(self.peek1(), b'-') {
            hint = serde_hint::I64;
            self.bump(1);
        }

        let mut wants_dot = true;
        let mut wants_e = true;
        let mut has_number = false;
        let mut any = false;

        loop {
            match self.peek1() {
                b'.' if wants_dot => {
                    hint = serde_hint::F64;
                    wants_dot = false;
                }
                b'e' | b'E' if has_number && wants_e => {
                    hint = serde_hint::F64;
                    wants_dot = false;
                    wants_e = false;
                }
                b'0'..=b'9' => {
                    has_number = true;
                }
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

        if tabular && !self.is_eol() {
            return None;
        }

        let string = self.data.insert_str(self.string(start));
        Some(Raw::Number(raw::Number::new(string, hint)))
    }

    /// Insert a null value as a placeholder.
    fn placeholder(&mut self, prefix: StringId, parent: Option<Id>) -> Id {
        self.data.insert(Raw::Null(Null::Empty), prefix, parent)
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
        let original = self.data.insert_str(self.string(original));

        Raw::String(raw::String::new(
            raw::RawStringKind::Original,
            string,
            original,
        ))
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

        Raw::String(raw::String::new(
            raw::RawStringKind::Original,
            string,
            original,
        ))
    }

    /// Read a double-quoted string.
    fn double_quoted(&mut self) -> Result<Raw> {
        let original = self.n;
        self.bump(1);
        let start = self.n;

        loop {
            match self.peek1() {
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
        let original = self.data.insert_str(self.string(original));

        Ok(Raw::String(raw::String::new(
            raw::RawStringKind::Original,
            string,
            original,
        )))
    }

    /// Parse a double quoted string.
    fn double_quoted_escaped(&mut self, start: usize, original: usize) -> Result<Raw> {
        self.scratch.extend(self.string(start));

        loop {
            match self.peek1() {
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

        Ok(Raw::String(raw::String::new(
            raw::RawStringKind::Original,
            string,
            original,
        )))
    }

    /// Unescape into the scratch buffer.
    fn unescape(&mut self, start: usize) -> Result<()> {
        let b = match self.peek1() {
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

            c |= match self.peek1() {
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
    fn inline_sequence(&mut self, prefix: StringId, parent: Option<Id>) -> Result<Id> {
        let id = self.placeholder(prefix, parent);

        self.bump(1);

        let mut items = Vec::new();
        let mut last = false;
        let mut trailing = false;
        let mut item_prefix = self.ws();

        while !matches!(self.peek1(), b']' | EOF) {
            trailing = false;

            let item_id = self.placeholder(item_prefix, Some(id));
            let value_prefix = self.ws();
            let (value, next_prefix) =
                self.value(value_prefix, Some(item_id), true, false, None)?;

            self.data.replace(item_id, raw::SequenceItem { value });

            items.push(item_id);

            if last {
                item_prefix = next_prefix.unwrap_or_else(|| self.ws());
                break;
            }

            if matches!(self.peek1(), b',') {
                self.bump(1);
                trailing = true;
            } else {
                last = true;
            }

            item_prefix = next_prefix.unwrap_or_else(|| self.ws());
        }

        if !matches!(self.peek1(), b']') {
            return Err(Error::new(
                self.span_back(item_prefix)..self.n,
                ErrorKind::BadSequenceTerminator,
            ));
        }

        self.bump(1);

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
        self.bump(1);

        let mut items = Vec::new();
        let mut last = false;
        let mut trailing = false;
        let mut start = self.n;
        let mut item_prefix = self.ws();

        while !matches!(self.peek1(), b'}' | EOF) {
            trailing = false;

            let Some(key) = self.until_colon(self.n) else {
                return Err(Error::new(start..self.n, ErrorKind::BadMappingSeparator));
            };

            let item_id = self.placeholder(item_prefix, Some(id));
            self.bump(1);
            let value_prefix = self.ws();
            let (value, next_prefix) =
                self.value(value_prefix, Some(item_id), true, false, None)?;

            self.data.replace(item_id, raw::MappingItem { key, value });
            items.push(item_id);

            item_prefix = next_prefix.unwrap_or_else(|| self.ws());

            if last {
                break;
            }

            if matches!(self.peek1(), b',') {
                self.bump(1);
                trailing = true;
            } else {
                last = true;
            }

            start = self.n;
            item_prefix = self.ws();
        }

        if !matches!(self.peek1(), b'}') {
            return Err(Error::new(start..self.n, ErrorKind::BadMappingTerminator));
        }

        self.bump(1);

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

            self.bump(1);

            let value_prefix = self.ws();
            let (value, ws) = self.value(value_prefix, Some(item_id), false, true, None)?;

            self.data.replace(item_id, raw::SequenceItem { value });
            items.push(item_id);

            let ws = ws.unwrap_or_else(|| self.ws());
            previous_ws = Some(ws);

            if self.indent() != indent || !matches!(self.peek1(), b'-') {
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
    ///
    /// If the indentation level is same as the parent, process this as a nul.
    fn mapping_or_nul(
        &mut self,
        mut start: usize,
        prefix: StringId,
        parent: Option<Id>,
        key: raw::String,
        parent_indent: Option<usize>,
    ) -> Result<(Id, Option<StringId>)> {
        let empty = self.data.insert_str("");
        let mapping_id = self.placeholder(prefix, parent);

        let mut items = Vec::new();
        let mut previous_ws = None;
        let mut current_key = Some(key);

        let indent = self.indent_from(start);

        // Finding a mapping on a smaller indentation level than the parent
        // indentation means that we've encountered a nul value and need to rewind parsing.
        if matches!(parent_indent, Some(i) if i >= indent) {
            self.n = start;
            self.data
                .replace_with(mapping_id, empty, Raw::Null(Null::Empty));
            return Ok((mapping_id, Some(prefix)));
        }

        while let Some(key) = current_key.take() {
            if !matches!(self.peek1(), b':') {
                self.bump(1);
                return Err(Error::new(start..self.n, ErrorKind::BadMappingSeparator));
            }

            let item_prefix = previous_ws.take().unwrap_or(empty);
            let item_id = self.placeholder(item_prefix, Some(mapping_id));

            self.bump(1);

            let value_prefix = self.ws();
            let (value, ws) = self.value(value_prefix, Some(item_id), false, true, Some(indent))?;

            self.data.replace(item_id, raw::MappingItem { key, value });
            items.push(item_id);

            let ws = ws.unwrap_or_else(|| self.ws());
            previous_ws = Some(ws);

            if self.indent() != indent {
                break;
            }

            start = self.n;
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
        self.indent_from(self.n)
    }

    /// Count indentation up until the current cursor.
    fn indent_from(&self, to: usize) -> usize {
        let string = self.input.get(..to).unwrap_or_default();
        raw::count_indent(string)
    }

    /// Process a key up until `:` or end of the current line.
    fn key_or_eol(&mut self, start: usize) -> Option<raw::String> {
        loop {
            self.find2(b':', raw::NEWLINE);
            let (a, b) = self.peek2();

            if matches!(a, EOF | raw::NEWLINE) {
                return None;
            }

            // Only treat something as a key if it's a colon immediately
            // followed by spacing.
            if a == b':' && is_spacing(b) {
                let key = self.data.insert_str(self.string(start));
                return Some(raw::String::new(raw::RawStringKind::Bare, key, key));
            }

            self.bump(1);
        }
    }

    /// Process a key up until `:`.
    fn until_colon(&mut self, start: usize) -> Option<raw::String> {
        while !matches!(self.peek1(), b':' | EOF) {
            self.bump(1);
        }

        if self.is_eof() {
            return None;
        }

        let key = self.data.insert_str(self.string(start));
        Some(raw::String::new(raw::RawStringKind::Bare, key, key))
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
        let start = self.n;
        self.bump(n);
        let prefix = self.data.insert_str(self.string(start));

        let start = self.n;
        let (mut ws, mut nl) = self.ws_nl();

        if nl == 0 {
            self.find(raw::NEWLINE);
            let out = self.input.get(start..self.n).unwrap_or_default().trim();
            self.scratch.extend_from_slice(out);

            (ws, nl) = self.ws_nl();

            if !self.is_eof() {
                self.scratch.push(join);
            }
        }

        let mut end = self.n;
        let indent = self.indent();

        while !self.is_eof() {
            let s = self.n;
            self.find(raw::NEWLINE);
            let out = self.input.get(s..self.n).unwrap_or_default().trim();
            self.scratch.extend_from_slice(out);

            end = self.n;
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

        let out = self.input.get(start..end).unwrap_or_default();
        let original = self.data.insert_str(out);

        let kind = raw::RawStringKind::Multiline { prefix };
        (
            Raw::String(raw::String::new(kind, string, original)),
            Some(ws),
        )
    }

    /// Consume a single value.
    fn value(
        &mut self,
        prefix: StringId,
        parent: Option<Id>,
        inline: bool,
        tabular: bool,
        parent_indent: Option<usize>,
    ) -> Result<(Id, Option<StringId>)> {
        let (raw, ws) = match self.peek2() {
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
                    let start = self.n;

                    if let Some(number) = self.number(start, tabular) {
                        break 'default (number, None);
                    }

                    if inline {
                        // Seek until we find a control character, since we're
                        // simply treating the current segment as a string.
                        while !matches!(self.peek1(), ctl!()) {
                            self.bump(1);
                        }
                    } else if let Some(key) = self.key_or_eol(start) {
                        return self.mapping_or_nul(start, prefix, parent, key, parent_indent);
                    }

                    // NB: calling `key_or_eol` will have consumed up until end
                    // of line for us, so use the current span as the production
                    // string.
                    match self.string(start) {
                        b"null" => (Raw::Null(Null::Keyword), None),
                        b"true" => (Raw::Boolean(true), None),
                        b"false" => (Raw::Boolean(false), None),
                        string => {
                            let string = self.data.insert_str(string);
                            let string = raw::String::new(raw::RawStringKind::Bare, string, string);
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
        let start = self.n;

        let string = loop {
            match self.peek1() {
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
        Some(raw::String::new(raw::RawStringKind::Bare, string, string))
    }
}

fn is_spacing(b: u8) -> bool {
    b.is_ascii_whitespace() || b == EOF
}
