use bstr::ByteSlice;

use crate::base;
use crate::serde_hint;
use crate::toml::data::{Data, Id, StringId};
use crate::toml::error::{Error, ErrorKind};
use crate::toml::raw::{self, Raw};
use crate::toml::Document;

type Result<T, E = Error> = std::result::Result<T, E>;

const EOF: u8 = b'\0';

/// Ascii whitespace matching.
macro_rules! ws {
    () => {
        b'\t' | raw::NEWLINE | b'\x0C' | b'\r' | raw::SPACE
    };
}

/// Ascii identifier continutation matching.
macro_rules! ident {
    () => {
        b'a'..=b'z' | b'_' | b'0'..=b'9'
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

    /// Parse an identifier.
    fn key(&mut self) -> Result<Option<raw::String>> {
        let start = self.parser.pos();

        if !matches!(self.parser.peek(), ident!()) {
            return Ok(None);
        }

        self.parser.bump(1);

        while matches!(self.parser.peek(), ident!()) {
            self.parser.bump(1);
        }

        let string = self.data.insert_str(self.parser.string(start));

        let raw = raw::String {
            kind: raw::RawStringKind::Bare,
            string,
        };

        Ok(Some(raw))
    }

    /// Parse a string identifier.
    fn sep(&mut self) -> Result<StringId> {
        let n = self.parser.pos();

        let mut no_sep = true;

        loop {
            match self.parser.peek() {
                ws!() => {}
                b'=' if no_sep => {
                    no_sep = false;
                }
                _ => {
                    break;
                }
            }

            self.parser.bump(1);
        }

        if no_sep {
            return Err(Error::new(
                n..self.parser.pos(),
                ErrorKind::ExpectedSeparator,
            ));
        }

        let sep = self.parser.string(n);
        Ok(self.data.insert_str(sep))
    }

    /// Parse a number.
    fn number(&mut self) -> Result<Id> {
        let start = self.parser.pos();
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
            return Err(Error::new(
                start..self.parser.pos(),
                ErrorKind::ExpectedNumber,
            ));
        }

        let string = self.data.insert_str(self.parser.string(start));
        let prefix = self.data.insert_str("");
        let id = self
            .data
            .insert(Raw::Number(raw::Number::new(string, hint)), prefix, None);
        Ok(id)
    }

    /// Parse a string.
    fn string(&mut self) -> Result<Id> {
        todo!()
    }

    /// Parse a TOML value.
    fn value(&mut self) -> Result<Id> {
        match self.parser.peek() {
            b'0'..=b'9' => self.number(),
            _ => self.string(),
        }
    }

    /// Parse a TOML table.
    fn table(&mut self) -> Result<(Id, StringId)> {
        let mut ws = self.ws();

        let id = self.data.insert(raw::Raw::Empty, ws, None);

        let mut items = Vec::new();

        while let Some(key) = self.key()? {
            let sep = self.sep()?;
            let value = self.value()?;

            let raw = raw::TableItem { key, sep, value };

            let raw = raw::Raw::TableItem(raw);
            let id = self.data.insert(raw, ws, None);
            items.push(id);

            ws = self.ws();
        }

        self.data.replace(id, raw::Raw::Table(raw::Table { items }));

        Ok((id, ws))
    }

    /// Parses a single value, and returns its kind.
    pub(crate) fn parse(mut self) -> Result<Document> {
        let (root, suffix) = self.table()?;

        if !self.parser.is_eof() {
            return Err(Error::new(self.parser.span(), ErrorKind::ExpectedEof));
        }

        Ok(Document::new(suffix, root, self.data))
    }
}
