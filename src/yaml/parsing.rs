use core::array;

use alloc::vec::Vec;

use crate::yaml::data::{Data, Id, StringId};
use crate::yaml::error::{Error, ErrorKind};
use crate::yaml::raw::{self, Raw};
use crate::yaml::serde_hint;
use crate::yaml::Document;

type Result<T, E = Error> = core::result::Result<T, E>;

const EOF: u8 = b'\0';

/// Ascii whitespace matching.
macro_rules! other_ws {
    ($($pat:pat_param)|*) => {
        b'\t' | b'\x0C' | b'\r' | raw::SPACE $(| $pat)*
    };
}

macro_rules! ws {
    ($($pat:pat_param)|*) => {
        other_ws!(raw::NEWLINE $(| $pat)*)
    };
}

/// Whitespace, a flow indicator or end of input.
macro_rules! ws_or_flow {
    () => {
        ws!(b',' | b'[' | b']' | b'{' | b'}' | EOF)
    };
}

struct State {
    prefix: StringId,
    parent: Option<Id>,
    inline: bool,
    tabular: bool,
    parent_indent: Option<usize>,
}

impl State {
    #[inline]
    fn new(prefix: StringId) -> Self {
        Self {
            prefix,
            parent: None,
            inline: false,
            tabular: false,
            parent_indent: None,
        }
    }

    #[inline]
    fn with_parent(self, parent: Id) -> Self {
        Self {
            parent: Some(parent),
            ..self
        }
    }

    #[inline]
    fn with_inline(self) -> Self {
        Self {
            inline: true,
            ..self
        }
    }

    #[inline]
    fn with_tabular(self) -> Self {
        Self {
            tabular: true,
            ..self
        }
    }

    #[inline]
    fn with_parent_indent(self, indent: usize) -> Self {
        Self {
            parent_indent: Some(indent),
            ..self
        }
    }
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

        let (root, suffix) = self.value(&State::new(prefix).with_tabular())?;

        let suffix = match suffix {
            Some(suffix) => suffix,
            None => self.ws(),
        };

        if !self.is_eof() {
            return Err(Error::new(self.n..self.input.len(), ErrorKind::ExpectedEof));
        }

        Ok(Document::from_parsed(suffix, root, self.data))
    }

    /// Process directives and the `---` document start marker.
    ///
    /// Everything up to the root value is kept verbatim as its prefix so that
    /// the header round-trips.
    fn start_of_document(&mut self) -> StringId {
        let start = self.n;
        self.ws();

        loop {
            match self.peek() {
                [b'%', _, _] => {
                    self.find(raw::NEWLINE);
                    self.ws();
                }
                [b'-', b'-', b'-'] => {
                    self.bump(3);
                    self.ws();
                    break;
                }
                _ => {
                    break;
                }
            }
        }

        self.data.insert_str(self.string(start))
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
                raw::NEWLINE => {
                    return true;
                }
                other_ws!() => {
                    n = n.wrapping_add(1);
                }
                _ => {
                    return false;
                }
            }
        }

        true
    }

    /// Consume a single number.
    fn number(&mut self, s: &State, start: usize) -> Option<Raw> {
        let mut hint = serde_hint::U64;

        if matches!(self.peek1(), b'-') {
            hint = serde_hint::I64;
            self.bump(1);
        }

        let mut wants_dot = true;
        let mut wants_e = true;
        let mut has_number = false;
        let mut has_exponent = true;

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
                    has_exponent = false;

                    if matches!(self.peek::<2>(), [_, b'-' | b'+']) {
                        self.bump(1);
                    }
                }
                b'0'..=b'9' => {
                    has_number = true;
                    has_exponent = true;
                }
                _ => {
                    break;
                }
            }

            self.bump(1);
        }

        // A mantissa digit and, after `e`, an exponent digit are required.
        if !has_number || !has_exponent {
            return None;
        }

        if s.tabular && !self.is_eol() {
            return None;
        }

        if s.inline && !self.is_flow_scalar_end() {
            return None;
        }

        let string = self.data.insert_str(self.string(start));
        Some(Raw::Number(raw::Number::new(string, hint)))
    }

    /// Insert a null value as a placeholder.
    fn placeholder(&mut self, prefix: StringId, parent: Option<Id>) -> Id {
        self.data
            .insert(Raw::Null(raw::Null::Empty), prefix, parent)
    }

    /// Error for a quoted string opened at `original` which reaches end of input.
    fn unterminated_string(&self, original: usize) -> Error {
        Error::new(original..self.input.len(), ErrorKind::UnterminatedString)
    }

    /// Read a single-quoted string.
    fn single_quoted(&mut self) -> Result<raw::String> {
        let original = self.n;
        self.bump(1);
        let start = self.n;

        loop {
            if self.is_eof() {
                return Err(self.unterminated_string(original));
            }

            match self.peek() {
                [b'\'', b'\''] => {
                    return self.single_quoted_escaped(start, original);
                }
                [b'\'', _] => {
                    break;
                }
                _ => {
                    self.bump(1);
                }
            }
        }

        let string = self.data.insert_str(self.string(start));
        self.bump(1);
        let original = self.data.insert_str(self.string(original));

        Ok(raw::String::new(
            raw::RawStringKind::Original,
            string,
            original,
        ))
    }

    /// Read a single-quoted escaped string.
    fn single_quoted_escaped(&mut self, start: usize, original: usize) -> Result<raw::String> {
        self.scratch.extend(self.string(start));

        loop {
            if self.is_eof() {
                return Err(self.unterminated_string(original));
            }

            match self.peek() {
                [b'\'', b'\''] => {
                    self.bump(2);
                    self.scratch.push(b'\'');
                }
                [b'\'', _] => {
                    break;
                }
                [b, _] => {
                    self.bump(1);
                    self.scratch.push(b);
                }
            }
        }

        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();
        self.bump(1);

        let original = self.data.insert_str(self.string(original));

        Ok(raw::String::new(
            raw::RawStringKind::Original,
            string,
            original,
        ))
    }

    /// Read a double-quoted string.
    fn double_quoted(&mut self) -> Result<raw::String> {
        let original = self.n;
        self.bump(1);
        let start = self.n;

        loop {
            if self.is_eof() {
                return Err(self.unterminated_string(original));
            }

            match self.peek1() {
                b'"' => break,
                b'\\' => {
                    return self.double_quoted_escaped(start, original);
                }
                _ => {
                    self.bump(1);
                }
            }
        }

        let string = self.data.insert_str(self.string(start));
        self.bump(1);
        let original = self.data.insert_str(self.string(original));

        Ok(raw::String::new(
            raw::RawStringKind::Original,
            string,
            original,
        ))
    }

    /// Parse a double quoted string.
    fn double_quoted_escaped(&mut self, start: usize, original: usize) -> Result<raw::String> {
        self.scratch.extend(self.string(start));

        loop {
            if self.is_eof() {
                return Err(self.unterminated_string(original));
            }

            match self.peek1() {
                b'"' => break,
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

        Ok(raw::String::new(
            raw::RawStringKind::Original,
            string,
            original,
        ))
    }

    /// Unescape into the scratch buffer.
    fn unescape(&mut self, start: usize) -> Result<()> {
        match self.peek1() {
            b'n' => {
                self.scratch.push(raw::NEWLINE);
            }
            b'0' => {
                self.scratch.push(b'\x00');
            }
            b'a' => {
                self.scratch.push(b'\x07');
            }
            b'b' => {
                self.scratch.push(b'\x08');
            }
            b't' | b'\x09' => {
                self.scratch.push(b'\x09');
            }
            b'v' => {
                self.scratch.push(b'\x0b');
            }
            b'f' => {
                self.scratch.push(b'\x0c');
            }
            b'r' => {
                self.scratch.push(b'\r');
            }
            b'e' => {
                self.scratch.push(b'\x1b');
            }
            b' ' => {
                self.scratch.push(b' ');
            }
            b'/' => {
                self.scratch.push(b'/');
            }
            b'\\' => {
                self.scratch.push(b'\\');
            }
            b'"' => {
                self.scratch.push(b'"');
            }
            b'N' => {
                self.scratch.extend("\u{0085}".as_bytes());
            }
            b'_' => {
                self.scratch.extend("\u{00A0}".as_bytes());
            }
            b'L' => {
                self.scratch.extend("\u{2028}".as_bytes());
            }
            b'P' => {
                self.scratch.extend("\u{2029}".as_bytes());
            }
            b'x' => {
                self.bump(1);
                return self.unescape_unicode(start, 2, ErrorKind::BadHexEscape);
            }
            b'u' => {
                self.bump(1);
                return self.unescape_unicode(start, 4, ErrorKind::BadUnicodeEscape);
            }
            b'U' => {
                self.bump(1);
                return self.unescape_unicode(start, 8, ErrorKind::BadUnicodeEscape);
            }
            _ => {
                self.bump(1);
                return Err(Error::new(start..self.n, ErrorKind::BadEscape));
            }
        }

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
    fn inline_sequence(&mut self, s: &State) -> Result<Id> {
        let id = self.placeholder(s.prefix, s.parent);

        self.bump(1);

        let mut items = Vec::new();
        let mut trailing = false;
        let mut item_prefix = self.ws();

        while !matches!(self.peek1(), b']' | EOF) {
            let item_id = self.placeholder(item_prefix, Some(id));
            let value_prefix = self.ws();
            let (value, next_prefix) =
                self.value(&State::new(value_prefix).with_parent(item_id).with_inline())?;

            self.data.replace(item_id, raw::SequenceItem { value });

            items.push(item_id);

            item_prefix = next_prefix.unwrap_or_else(|| self.ws());
            trailing = matches!(self.peek1(), b',');

            if !trailing {
                break;
            }

            self.bump(1);
            item_prefix = self.ws();
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
    fn inline_mapping(&mut self, s: &State) -> Result<Id> {
        let id = self.placeholder(s.prefix, s.parent);
        self.bump(1);

        let mut items = Vec::new();
        let mut trailing = false;
        let mut start = self.n;
        let mut item_prefix = self.ws();

        while !matches!(self.peek1(), b'}' | EOF) {
            let Some(key) = self.until_colon(self.n) else {
                return Err(Error::new(start..self.n, ErrorKind::BadMappingSeparator));
            };

            let item_id = self.placeholder(item_prefix, Some(id));
            self.bump(1);
            let value_prefix = self.ws();
            let (value, next_prefix) =
                self.value(&State::new(value_prefix).with_parent(item_id).with_inline())?;

            self.data.replace(item_id, raw::MappingItem { key, value });
            items.push(item_id);

            item_prefix = next_prefix.unwrap_or_else(|| self.ws());
            trailing = matches!(self.peek1(), b',');

            if !trailing {
                break;
            }

            self.bump(1);
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
    fn sequence(&mut self, s: &State) -> Result<(Id, Option<StringId>)> {
        let empty = self.data.insert_str("");
        let mapping_id = self.placeholder(s.prefix, s.parent);

        let mut items = Vec::new();
        let mut previous_ws = None;
        let indent = self.indent();

        loop {
            let item_prefix = previous_ws.take().unwrap_or(empty);
            let item_id = self.placeholder(item_prefix, Some(mapping_id));

            self.bump(1);

            let value_prefix = self.ws();
            let (value, ws) = self.value(
                &State::new(value_prefix)
                    .with_parent(item_id)
                    .with_tabular()
                    .with_parent_indent(indent),
            )?;

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
        s: &State,
        mut start: usize,
        key: raw::String,
    ) -> Result<(Id, Option<StringId>)> {
        let empty = self.data.insert_str("");
        let mapping_id = self.placeholder(s.prefix, s.parent);

        let mut items = Vec::new();
        let mut previous_ws = None;
        let mut current_key = Some(key);

        let indent = self.indent_from(start);

        // Finding a mapping on a smaller indentation level than the parent
        // indentation means that we've encountered a nul value and need to rewind parsing.
        if matches!(s.parent_indent, Some(i) if i >= indent) {
            self.n = start;
            self.data
                .replace_with(mapping_id, empty, Raw::Null(raw::Null::Empty));
            return Ok((mapping_id, Some(s.prefix)));
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
            let (value, ws) = self.value(
                &State::new(value_prefix)
                    .with_parent(item_id)
                    .with_tabular()
                    .with_parent_indent(indent),
            )?;

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

            match self.peek() {
                [raw::NEWLINE | EOF, _] => {
                    return None;
                }
                // Only treat something as a key if it's a colon immediately
                // followed by spacing.
                [b':', ws!(EOF)] => {
                    let key = self.data.insert_str(self.string(start));
                    return Some(raw::String::new(raw::RawStringKind::Bare, key, key));
                }
                _ => {
                    self.bump(1);
                }
            }
        }
    }

    /// Process a flow mapping key up until its `:` separator.
    ///
    /// Returns `None` if the key ends at `,`, `}` or end of input instead.
    fn until_colon(&mut self, start: usize) -> Option<raw::String> {
        let quote = match self.input.get(start) {
            Some(&q @ (b'"' | b'\'')) => Some(q),
            _ => None,
        };

        loop {
            match self.peek() {
                [b',' | b'}' | EOF, _] => return None,
                [b':', ws_or_flow!()] => break,
                // JSON-like keys such as `{"a":1}`.
                [b':', _]
                    if self.n > start.wrapping_add(1)
                        && quote.is_some()
                        && self.input.get(self.n.wrapping_sub(1)).copied() == quote =>
                {
                    break;
                }
                _ => self.bump(1),
            }
        }

        let key = self.data.insert_str(self.string(start));
        Some(raw::String::new(raw::RawStringKind::Bare, key, key))
    }

    /// Test if a flow scalar ends at the current position, optionally after
    /// whitespace or a comment.
    fn is_flow_scalar_end(&self) -> bool {
        let mut n = self.n;

        while let Some(ws!()) = self.input.get(n).copied() {
            n = n.wrapping_add(1);
        }

        let at = |n: usize| self.input.get(n).copied().unwrap_or(EOF);

        match at(n) {
            b',' | b']' | b'}' | EOF => true,
            b':' => matches!(at(n.wrapping_add(1)), ws_or_flow!()),
            b'#' => n > self.n,
            _ => false,
        }
    }

    /// Process a block as a string.
    ///
    /// Content lines must be indented deeper than `parent_indent`.
    fn block(
        &mut self,
        n: usize,
        join: u8,
        folded: bool,
        chomp: bool,
        clip: bool,
        parent_indent: Option<usize>,
    ) -> (Raw, Option<StringId>) {
        let start = self.n;
        self.bump(n);
        let prefix = self.data.insert_str(self.string(start));

        let start = self.n;
        let mut end = self.n;
        let mut content = false;
        let (mut ws, mut nl) = self.ws_nl();

        if nl == 0 {
            self.find(raw::NEWLINE);
            let out = trim(self.input.get(start..self.n).unwrap_or_default());
            self.scratch.extend_from_slice(out);
            content = true;
            end = self.n;
            (ws, nl) = self.ws_nl();
        }

        let indent = self.indent();
        let deeper = !self.is_eof() && parent_indent.map_or(true, |p| indent > p);

        if deeper && content {
            self.scratch.push(join);
        }

        content |= deeper;

        while deeper && !self.is_eof() {
            let s = self.n;
            self.find(raw::NEWLINE);
            let out = trim(self.input.get(s..self.n).unwrap_or_default());
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

        if content {
            for _ in 0..if chomp { nl } else { Default::default() } {
                self.scratch.push(raw::NEWLINE);

                if clip {
                    break;
                }
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
    fn value(&mut self, s: &State) -> Result<(Id, Option<StringId>)> {
        let (raw, ws) = match self.peek() {
            [b'-', ws!()] if !s.inline => {
                return self.sequence(s);
            }
            [b'"', _] => {
                let start = self.n;
                let string = self.double_quoted()?;

                if !s.inline && self.peek1() == b':' {
                    return self.mapping_or_nul(s, start, string);
                }

                (Raw::String(string), None)
            }
            [b'\'', _] => {
                let start = self.n;
                let string = self.single_quoted()?;

                if !s.inline && self.peek1() == b':' {
                    return self.mapping_or_nul(s, start, string);
                }

                (Raw::String(string), None)
            }
            [b'[', _] => return Ok((self.inline_sequence(s)?, None)),
            [b'{', _] => return Ok((self.inline_mapping(s)?, None)),
            [a @ (b'>' | b'|'), b] => self.block(
                matches!(b, b'-' | b'+').then_some(2).unwrap_or(1),
                if a == b'>' { raw::SPACE } else { raw::NEWLINE },
                a == b'>',
                b != b'-',
                b == b'-',
                s.parent_indent,
            ),
            _ => {
                'default: {
                    let start = self.n;

                    if let Some(number) = self.number(s, start) {
                        break 'default (number, None);
                    }

                    if s.inline {
                        // A `:` only ends a flow plain scalar when followed by
                        // whitespace or a flow indicator, so `http://x` stays
                        // whole.
                        loop {
                            match self.peek() {
                                [b',' | b']' | b'}' | EOF, _] | [b':', ws_or_flow!()] => break,
                                _ => self.bump(1),
                            }
                        }
                    } else {
                        if let Some(key) = self.key_or_eol(start) {
                            return self.mapping_or_nul(s, start, key);
                        }

                        // key_or_eol consumed up until end of line.
                        // Check for plain multi-line scalar continuation
                        // lines indented deeper than the parent mapping.
                        if let Some(parent_indent) = s.parent_indent {
                            loop {
                                if self.peek1() != raw::NEWLINE {
                                    break;
                                }
                                let saved = self.n;
                                self.bump(1);
                                while matches!(self.peek1(), other_ws!()) {
                                    self.bump(1);
                                }
                                if self.is_eof()
                                    || self.peek1() == raw::NEWLINE
                                    || self.indent() <= parent_indent
                                {
                                    self.n = saved;
                                    break;
                                }
                                self.find(raw::NEWLINE);
                            }
                        }
                    }

                    let string = self.string(start);

                    if memchr::memchr(raw::NEWLINE, string).is_some() {
                        // Multi-line plain scalar: fold lines with spaces
                        for line in string.split(|&b| b == raw::NEWLINE) {
                            let trimmed = trim(line);
                            if trimmed.is_empty() {
                                continue;
                            }
                            if !self.scratch.is_empty() {
                                self.scratch.push(raw::SPACE);
                            }
                            self.scratch.extend_from_slice(trimmed);
                        }
                        let folded = self.data.insert_str(&self.scratch);
                        self.scratch.clear();
                        let original = self.data.insert_str(string);
                        break 'default (
                            Raw::String(raw::String::new(
                                raw::RawStringKind::Original,
                                folded,
                                original,
                            )),
                            None,
                        );
                    }

                    (
                        if string == b"~" {
                            Raw::Null(raw::Null::Tilde)
                        } else if string.eq_ignore_ascii_case(b"null") {
                            Raw::Null(raw::Null::Keyword(self.data.insert_str(string)))
                        } else if string.eq_ignore_ascii_case(b"true") {
                            Raw::Boolean(raw::Boolean::new(true, self.data.insert_str(string)))
                        } else if string.eq_ignore_ascii_case(b"false") {
                            Raw::Boolean(raw::Boolean::new(false, self.data.insert_str(string)))
                        } else {
                            let string = self.data.insert_str(string);
                            Raw::String(raw::String::new(raw::RawStringKind::Bare, string, string))
                        },
                        None,
                    )
                }
            }
        };

        let value = self.data.insert(raw, s.prefix, s.parent);
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

/// Trim ascii whitespace.
const fn trim(bytes: &[u8]) -> &[u8] {
    let mut out = bytes;

    while let [ws!(), rest @ ..] = out {
        out = rest;
    }

    while let [rest @ .., ws!()] = out {
        out = rest;
    }

    out
}
