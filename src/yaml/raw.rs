use std::fmt::{self, Write};

use bstr::ByteSlice;

use crate::slab::Pointer;
use crate::strings::StringId;
use crate::yaml::{Document, NullKind, StringKind};

/// A raw value.
#[derive(Debug, Clone)]
pub(crate) enum Raw {
    /// A null value.
    Null(NullKind),
    /// A single number.
    Number(RawNumber),
    /// A string.
    String(RawString),
    /// A table.
    #[allow(unused)]
    Table(RawTable),
}

impl Raw {
    pub(crate) fn display(&self, doc: &Document, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::fmt::Display;

        match self {
            Raw::Null(raw) => {
                match raw {
                    NullKind::Keyword => {
                        "null".fmt(f)?;
                    }
                    NullKind::Tilde => {
                        "~".fmt(f)?;
                    }
                    NullKind::Empty => {
                        // empty values count as null.
                    }
                }
            }
            Raw::Number(raw) => {
                doc.strings.get(&raw.string).fmt(f)?;
            }
            Raw::String(raw) => {
                let string = doc.strings.get(&raw.string);

                match raw.kind {
                    StringKind::Bare => {
                        string.fmt(f)?;
                    }
                    StringKind::DoubleQuoted => {
                        escape_double_quoted(string, f)?;
                    }
                    StringKind::SingleQuoted => {
                        escape_single_quoted(string, f)?;
                    }
                }
            }
            Raw::Table(table) => {
                for e in &table.children {
                    if let Some(prefix) = &e.prefix {
                        doc.strings.get(prefix).fmt(f)?;
                    }

                    doc.strings.get(&e.key.string).fmt(f)?;
                    ':'.fmt(f)?;
                    doc.strings.get(&e.separator).fmt(f)?;

                    if let Some(raw) = doc.tree.get(&e.value) {
                        raw.display(doc, f)?;
                    }
                }
            }
        }

        Ok(())
    }
}

/// Single-quoted escape sequences:
/// <https://yaml.org/spec/1.2.2/#escaped-characters>.
fn escape_single_quoted(string: &bstr::BStr, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    f.write_char('\'')?;

    for c in string.chars() {
        match c {
            '\'' => {
                f.write_str("''")?;
            }
            c => {
                f.write_char(c)?;
            }
        }
    }

    f.write_char('\'')?;
    Ok(())
}

/// Double-quoted escape sequences:
/// <https://yaml.org/spec/1.2.2/#escaped-characters>.
fn escape_double_quoted(string: &bstr::BStr, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    f.write_char('"')?;

    for c in string.chars() {
        match c {
            '\u{0000}' => {
                f.write_str("\\0")?;
            }
            '\u{0007}' => {
                f.write_str("\\a")?;
            }
            '\u{0008}' => {
                f.write_str("\\b")?;
            }
            '\u{0009}' => {
                f.write_str("\\t")?;
            }
            '\n' => {
                f.write_str("\\n")?;
            }
            '\u{000b}' => {
                f.write_str("\\v")?;
            }
            '\u{000c}' => {
                f.write_str("\\f")?;
            }
            '\r' => {
                f.write_str("\\r")?;
            }
            '\u{001b}' => {
                f.write_str("\\e")?;
            }
            '\"' => {
                f.write_str("\\\"")?;
            }
            c if c.is_ascii_control() => {
                write!(f, "\\x{:02x}", c as u8)?;
            }
            c => {
                f.write_char(c)?;
            }
        }
    }

    f.write_char('"')?;
    Ok(())
}

/// A YAML number.
#[derive(Debug, Clone)]
pub(crate) struct RawNumber {
    pub(crate) string: StringId,
}

impl RawNumber {
    /// A simple number.
    pub(crate) fn new(string: StringId) -> Self {
        Self { string }
    }
}

/// A YAML string.
#[derive(Debug, Clone)]
pub(crate) struct RawString {
    pub(crate) kind: StringKind,
    pub(crate) string: StringId,
}

impl RawString {
    /// A simple number.
    pub(crate) fn new(kind: StringKind, string: StringId) -> Self {
        Self { kind, string }
    }
}

/// An element in a YAML table.
#[derive(Debug, Clone)]
pub(crate) struct RawTableElement {
    pub(crate) prefix: Option<StringId>,
    pub(crate) key: RawString,
    pub(crate) separator: StringId,
    pub(crate) value: Pointer,
}

/// A YAML table.
#[derive(Debug, Clone)]
pub(crate) struct RawTable {
    pub(crate) indent: StringId,
    pub(crate) children: Vec<RawTableElement>,
}

impl RawTable {
    /// Insert a value into the table.
    pub(crate) fn insert(&mut self, key: RawString, separator: StringId, value: Pointer) {
        if let Some(existing) = self
            .children
            .iter_mut()
            .find(|c| c.key.string == key.string)
        {
            existing.separator = separator;
            existing.value = value;
            return;
        }

        let prefix = if !self.children.is_empty() {
            Some(self.indent)
        } else {
            None
        };

        self.children.push(RawTableElement {
            prefix,
            key,
            separator,
            value,
        });
    }
}
