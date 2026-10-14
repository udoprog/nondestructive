use core::{array, iter, mem};

use alloc::vec::Vec;

use crate::yaml::data::{Data, Id, StringId};
use crate::yaml::error::{Error, ErrorKind};
use crate::yaml::number;
use crate::yaml::raw::{self, Raw};
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

/// Block scalar chomping.
#[derive(Clone, Copy)]
enum Chomp {
    Strip,
    Clip,
    Keep,
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

    /// Peek the next `N` values.
    fn peek<const N: usize>(&self) -> [u8; N] {
        array::from_fn(|n| {
            self.input
                .get(self.n.wrapping_add(n))
                .copied()
                .unwrap_or(EOF)
        })
    }

    /// Bump `n` bytes of input.
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

    /// Test if the cursor follows a space or tab, so that a `#` there starts
    /// a comment.
    fn follows_ws(&self) -> bool {
        matches!(
            self.input.get(self.n.wrapping_sub(1)),
            Some(&(raw::SPACE | b'\t'))
        )
    }

    /// Advance to the end of a plain scalar's line: a line break, end of
    /// input or the `#` of a comment. Returns `true` at a comment.
    fn plain_line_end(&mut self) -> bool {
        loop {
            self.find2(b'#', raw::NEWLINE);

            match self.peek1() {
                b'#' if self.follows_ws() => return true,
                b'#' => self.bump(1),
                _ => return false,
            }
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

    /// Test if current position contains nothing but whitespace until we
    /// reach a line end or a comment.
    fn is_eol(&self) -> bool {
        let mut n = self.n;

        while let Some(&b) = self.input.get(n) {
            match b {
                raw::NEWLINE => {
                    return true;
                }
                b'#' => {
                    return n > self.n;
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

    /// Consume a plain scalar that resolves to a number.
    fn number(&mut self, s: &State, start: usize) -> Option<Raw> {
        while matches!(self.peek1(), b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'.' | b'+' | b'-') {
            self.bump(1);
        }

        let hint = number::resolve(self.string(start))?;

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
                [b'\'', b'\''] | [raw::NEWLINE | b'\r', _] => {
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

            if self.line_break_len() > 0 {
                self.fold(0);
                continue;
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
                b'\\' | raw::NEWLINE | b'\r' => {
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
        // Whitespace produced by escapes is content and must survive folding.
        let mut protected = 0;

        loop {
            if self.is_eof() {
                return Err(self.unterminated_string(original));
            }

            if self.line_break_len() > 0 {
                self.fold(protected);
                continue;
            }

            match self.peek1() {
                b'"' => break,
                b'\\' => {
                    let start = self.n;
                    self.bump(1);
                    self.unescape(start)?;
                    protected = self.scratch.len();
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

    /// Length of the line break at the cursor, or 0 if there is none.
    fn line_break_len(&self) -> usize {
        match self.peek() {
            [raw::NEWLINE, _] => 1,
            [b'\r', raw::NEWLINE] => 2,
            _ => 0,
        }
    }

    /// Skip the line break at the cursor and any empty lines after it, along
    /// with the leading whitespace of the following line. Returns the number
    /// of line breaks skipped.
    fn skip_line_breaks(&mut self) -> usize {
        let mut count = 0usize;

        loop {
            let len = self.line_break_len();

            if len == 0 {
                return count;
            }

            self.bump(len);
            count = count.wrapping_add(1);

            while matches!(self.peek1(), b' ' | b'\t') {
                self.bump(1);
            }
        }
    }

    /// Fold a line break inside a quoted scalar into the scratch buffer.
    ///
    /// Trailing whitespace before the break is dropped, except the first
    /// `protected` bytes of scratch. A single break becomes a space and each
    /// further empty line a newline.
    fn fold(&mut self, protected: usize) {
        while self.scratch.len() > protected && matches!(self.scratch.last(), Some(b' ' | b'\t')) {
            self.scratch.pop();
        }

        match self.skip_line_breaks() {
            1 => self.scratch.push(b' '),
            n => self
                .scratch
                .extend(iter::repeat(raw::NEWLINE).take(n.saturating_sub(1))),
        }
    }

    /// Unescape into the scratch buffer.
    fn unescape(&mut self, start: usize) -> Result<()> {
        if self.line_break_len() > 0 {
            let n = self.skip_line_breaks();
            self.scratch
                .extend(iter::repeat(raw::NEWLINE).take(n.saturating_sub(1)));
            return Ok(());
        }

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

            items.push(item_id);

            let ws = next_prefix.unwrap_or_else(|| self.ws());
            trailing = matches!(self.peek1(), b',');

            let separator_prefix = if trailing {
                ws
            } else {
                item_prefix = ws;
                self.data.insert_str("")
            };

            self.data.replace(
                item_id,
                raw::SequenceItem {
                    value,
                    separator_prefix,
                },
            );

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
            let Some(key) = self.until_colon(self.n)? else {
                return Err(Error::new(start..self.n, ErrorKind::BadMappingSeparator));
            };

            let item_id = self.placeholder(item_prefix, Some(id));
            self.bump(1);
            let value_prefix = self.ws();
            let (value, next_prefix) =
                self.value(&State::new(value_prefix).with_parent(item_id).with_inline())?;

            items.push(item_id);

            let ws = next_prefix.unwrap_or_else(|| self.ws());
            trailing = matches!(self.peek1(), b',');

            let separator_prefix = if trailing {
                ws
            } else {
                item_prefix = ws;
                self.data.insert_str("")
            };

            self.data.replace(
                item_id,
                raw::MappingItem {
                    key,
                    value,
                    separator_prefix,
                },
            );

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

            let (value, ws) = if self.is_empty_value(value_prefix, indent, true) {
                self.empty_value(value_prefix, item_id)
            } else {
                self.value(
                    &State::new(value_prefix)
                        .with_parent(item_id)
                        .with_tabular()
                        .with_parent_indent(indent),
                )?
            };

            self.data.replace(
                item_id,
                raw::SequenceItem {
                    value,
                    separator_prefix: empty,
                },
            );
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
            let separator = self.data.insert_str(" ");
            self.data
                .replace_with(mapping_id, separator, Raw::Null(raw::Null::Empty));
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

            let (value, ws) = if self.is_empty_value(value_prefix, indent, false) {
                self.empty_value(value_prefix, item_id)
            } else {
                self.value(
                    &State::new(value_prefix)
                        .with_parent(item_id)
                        .with_tabular()
                        .with_parent_indent(indent),
                )?
            };

            self.data.replace(
                item_id,
                raw::MappingItem {
                    key,
                    value,
                    separator_prefix: empty,
                },
            );
            items.push(item_id);

            let ws = ws.unwrap_or_else(|| self.ws());
            previous_ws = Some(ws);

            if self.indent() != indent {
                break;
            }

            start = self.n;
            current_key = self.next_mapping_key()?;
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

    /// Test if the value of a block collection entry at `indent`, preceded by
    /// `prefix`, is empty: it ends at end of input or at a line which belongs
    /// to a sibling or an ancestor. A mapping value may be a sequence at the
    /// same indentation as its key.
    fn is_empty_value(&self, prefix: StringId, indent: usize, sequence: bool) -> bool {
        if self.is_eof() {
            return true;
        }

        if memchr::memchr(raw::NEWLINE, self.data.str(prefix)).is_none() {
            return false;
        }

        let current = self.indent();
        current < indent
            || current == indent && (sequence || !matches!(self.peek(), [b'-', ws!(EOF)]))
    }

    /// Construct an empty value. Its prefix is handed back as trailing
    /// whitespace, since an empty value outputs nothing; the value instead gets
    /// the separator a later edit prints before it.
    fn empty_value(&mut self, prefix: StringId, parent: Id) -> (Id, Option<StringId>) {
        let separator = self.data.insert_str(" ");
        let id = self.placeholder(separator, Some(parent));
        (id, Some(prefix))
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

    /// Process a plain block key up until a `:` followed by whitespace or end
    /// of input.
    ///
    /// Returns `None` with the cursor at the end of the line or at the `#` of a
    /// comment if the line has no such `:` before either.
    fn key_or_eol(&mut self, start: usize) -> Option<raw::String> {
        loop {
            let input = self.input.get(self.n..).unwrap_or_default();

            match memchr::memchr3(b':', b'#', raw::NEWLINE, input) {
                Some(n) => self.bump(n),
                None => self.n = self.input.len(),
            }

            match self.peek() {
                [raw::NEWLINE | EOF, _] => return None,
                [b'#', _] if self.follows_ws() => return None,
                [b':', ws!(EOF)] => break,
                _ => self.bump(1),
            }
        }

        Some(self.plain_key(start))
    }

    /// Build a plain key from `start` to the cursor, excluding trailing
    /// whitespace from its value but keeping it in its original form.
    fn plain_key(&mut self, start: usize) -> raw::String {
        let original = self.string(start);
        let trimmed = trim_end(original);
        let key = self.data.insert_str(trimmed);

        if trimmed.len() == original.len() {
            return raw::String::new(raw::RawStringKind::Bare, key, key);
        }

        let original = self.data.insert_str(original);
        raw::String::new(raw::RawStringKind::Original, key, original)
    }

    /// Treat a just-read quoted string which started at `start` as a key if
    /// it is followed by `:`, optionally after spaces.
    ///
    /// Otherwise the cursor is left after the closing quote.
    fn quoted_key(&mut self, start: usize, string: &raw::String) -> Option<raw::String> {
        let end = self.n;

        while matches!(self.peek1(), other_ws!()) {
            self.bump(1);
        }

        if self.peek1() != b':' {
            self.n = end;
            return None;
        }

        if self.n == end {
            return Some(string.clone());
        }

        let original = self.data.insert_str(self.string(start));
        Some(raw::String::new(
            raw::RawStringKind::Original,
            string.id,
            original,
        ))
    }

    /// Process a flow mapping key up until its `:` separator.
    ///
    /// Returns `None` if the key ends at `,`, `}` or end of input instead.
    fn until_colon(&mut self, start: usize) -> Result<Option<raw::String>> {
        let string = match self.peek1() {
            b'"' => self.double_quoted()?,
            b'\'' => self.single_quoted()?,
            _ => {
                loop {
                    match self.peek() {
                        [b',' | b'}' | EOF, _] => return Ok(None),
                        [b':', ws_or_flow!()] => break,
                        _ => self.bump(1),
                    }
                }

                return Ok(Some(self.plain_key(start)));
            }
        };

        Ok(self.quoted_key(start, &string))
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

    /// Process a block scalar starting at its `|` or `>` indicator.
    ///
    /// Content lines must be indented deeper than `parent_indent`. As an
    /// extension, text after the header is taken as the first line, joined to
    /// the body by a single line break (or space when folded).
    fn block(&mut self, parent_indent: Option<usize>) -> (Raw, Option<StringId>) {
        let start = self.n;
        let folded = self.peek1() == b'>';
        self.bump(1);

        let mut chomp = None;
        let mut explicit = None;

        loop {
            match self.peek1() {
                b'-' if chomp.is_none() => chomp = Some(Chomp::Strip),
                b'+' if chomp.is_none() => chomp = Some(Chomp::Keep),
                b @ b'1'..=b'9' if explicit.is_none() => explicit = Some(usize::from(b - b'0')),
                _ => break,
            }

            self.bump(1);
        }

        let chomp = chomp.unwrap_or(Chomp::Clip);
        let prefix = self.data.insert_str(self.string(start));

        let start = self.n;
        let mut end = start;
        // Pending line breaks since the last content line.
        let mut breaks = 0usize;
        // Whether the last content line was more indented, if there was one.
        let mut spaced = None;
        let mut header = false;

        while matches!(self.peek1(), other_ws!()) {
            self.bump(1);
        }

        match self.peek1() {
            b'#' => self.find(raw::NEWLINE),
            raw::NEWLINE | EOF => {}
            _ => {
                let s = self.n;
                self.find(raw::NEWLINE);
                self.scratch.extend_from_slice(trim(self.string(s)));
                end = self.n;
                header = true;
                spaced = Some(false);
            }
        }

        if self.peek1() == raw::NEWLINE {
            self.bump(1);
            breaks = usize::from(header);
        }

        let min = parent_indent.map_or(0, |p| p.wrapping_add(1));

        let indent = match explicit {
            Some(d) => parent_indent.map_or(d, |p| p.wrapping_add(d)),
            None => self.detect_block_indent(),
        }
        .max(min);

        while !self.is_eof() {
            let line = self.n;

            while self.n.wrapping_sub(line) < indent && matches!(self.peek1(), raw::SPACE | b'\t') {
                self.bump(1);
            }

            let text_start = self.n;
            let shallow = text_start.wrapping_sub(line) < indent;
            self.find(raw::NEWLINE);
            let text = self.string(text_start);
            let text = text.strip_suffix(b"\r").unwrap_or(text);

            if text.is_empty() || shallow {
                if !text.iter().all(|&b| matches!(b, other_ws!())) {
                    break;
                }

                if self.peek1() == raw::NEWLINE {
                    self.bump(1);
                    breaks = breaks.wrapping_add(1);
                }

                continue;
            }

            let is_spaced = matches!(text.first(), Some(b' ' | b'\t'));

            match spaced {
                Some(_) if mem::take(&mut header) => {
                    self.scratch
                        .push(if folded { raw::SPACE } else { raw::NEWLINE });
                }
                Some(false) if folded && !is_spaced => {
                    if breaks == 1 {
                        self.scratch.push(raw::SPACE);
                    } else {
                        let n = breaks.saturating_sub(1);
                        self.scratch.extend(iter::repeat(raw::NEWLINE).take(n));
                    }
                }
                _ => {
                    self.scratch.extend(iter::repeat(raw::NEWLINE).take(breaks));
                }
            }

            self.scratch.extend_from_slice(text);
            end = self.n;
            spaced = Some(is_spaced);
            breaks = 0;

            if self.peek1() == raw::NEWLINE {
                self.bump(1);
                breaks = 1;
            }
        }

        let trailing = match chomp {
            Chomp::Clip if spaced.is_some() => breaks.min(1),
            Chomp::Strip | Chomp::Clip => 0,
            Chomp::Keep => breaks,
        };

        self.scratch
            .extend(iter::repeat(raw::NEWLINE).take(trailing));

        let string = self.data.insert_str(&self.scratch);
        self.scratch.clear();

        let original = self
            .data
            .insert_str(self.input.get(start..end).unwrap_or_default());

        self.n = end;
        let ws = self.ws();

        let kind = raw::RawStringKind::Multiline { prefix };
        (
            Raw::String(raw::String::new(kind, string, original)),
            Some(ws),
        )
    }

    /// Detect the content indentation of a block scalar from the line at the
    /// cursor: that of the first non-empty line, or of a deeper preceding
    /// empty line.
    fn detect_block_indent(&self) -> usize {
        let mut max = 0;

        for line in self
            .input
            .get(self.n..)
            .unwrap_or_default()
            .split(|&b| b == raw::NEWLINE)
        {
            let spaces = line
                .iter()
                .take_while(|&&b| matches!(b, raw::SPACE | b'\t'))
                .count();
            let rest = line.get(spaces..).unwrap_or_default();

            if !rest.iter().all(|&b| matches!(b, other_ws!())) {
                return max.max(spaces);
            }

            max = max.max(spaces);
        }

        max
    }

    /// Consume a single value.
    fn value(&mut self, s: &State) -> Result<(Id, Option<StringId>)> {
        let (raw, ws) = match self.peek() {
            [EOF, _] if !s.inline && self.is_eof() => (Raw::Null(raw::Null::Empty), None),
            [b'-', ws!(EOF)] if !s.inline => {
                return self.sequence(s);
            }
            [b'"', _] => {
                let start = self.n;
                let string = self.double_quoted()?;

                if !s.inline {
                    if let Some(key) = self.quoted_key(start, &string) {
                        return self.mapping_or_nul(s, start, key);
                    }
                }

                (Raw::String(string), None)
            }
            [b'\'', _] => {
                let start = self.n;
                let string = self.single_quoted()?;

                if !s.inline {
                    if let Some(key) = self.quoted_key(start, &string) {
                        return self.mapping_or_nul(s, start, key);
                    }
                }

                (Raw::String(string), None)
            }
            [b'[', _] => return Ok((self.inline_sequence(s)?, None)),
            [b'{', _] => return Ok((self.inline_mapping(s)?, None)),
            [b'>' | b'|', _] => self.block(s.parent_indent),
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
                                [b'#', _] if self.follows_ws() => break,
                                _ => self.bump(1),
                            }
                        }
                    } else {
                        if let Some(key) = self.key_or_eol(start) {
                            return self.mapping_or_nul(s, start, key);
                        }

                        // Continuation lines indented deeper than the parent
                        // extend the scalar; a comment ends it.
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
                                    || matches!(self.peek1(), raw::NEWLINE | b'#')
                                    || self.indent() <= parent_indent
                                {
                                    self.n = saved;
                                    break;
                                }
                                if self.plain_line_end() {
                                    break;
                                }
                            }
                        }
                    }

                    // Trailing whitespace is left to the following whitespace.
                    self.n = start.wrapping_add(trim_end(self.string(start)).len());
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
                        } else if raw::NULL.contains(&string) {
                            Raw::Null(raw::Null::Keyword(self.data.insert_str(string)))
                        } else if raw::TRUE.contains(&string) {
                            Raw::Boolean(raw::Boolean::new(true, self.data.insert_str(string)))
                        } else if raw::FALSE.contains(&string) {
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

    /// Parse the next key of a block mapping, or `None` at end of input or a
    /// line starting with `:`.
    fn next_mapping_key(&mut self) -> Result<Option<raw::String>> {
        let start = self.n;

        let key = match self.peek1() {
            b':' | EOF => return Ok(None),
            b'"' => {
                let string = self.double_quoted()?;
                self.quoted_key(start, &string)
            }
            b'\'' => {
                let string = self.single_quoted()?;
                self.quoted_key(start, &string)
            }
            _ => self.key_or_eol(start),
        };

        if let Some(key) = key {
            return Ok(Some(key));
        }

        let end = start.wrapping_add(trim_end(self.string(start)).len());
        Err(Error::new(start..end, ErrorKind::BadMappingSeparator))
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

/// Trim trailing ascii whitespace.
const fn trim_end(bytes: &[u8]) -> &[u8] {
    let mut out = bytes;

    while let [rest @ .., ws!()] = out {
        out = rest;
    }

    out
}
