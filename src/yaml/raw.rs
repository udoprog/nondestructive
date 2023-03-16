use core::fmt;

use crate::slab::Pointer;
use crate::strings::StringId;
use crate::yaml::{Document, NullKind, StringKind};

/// A raw value.
#[derive(Debug, Clone)]
pub(crate) struct Raw {
    #[allow(unused)]
    pub(crate) pointer: Pointer,
    pub(crate) kind: RawKind,
}

impl Raw {
    pub(crate) fn new(pointer: Pointer, kind: RawKind) -> Self {
        Self { pointer, kind }
    }

    pub(crate) fn display(&self, doc: &Document, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::fmt::Display;

        match &self.kind {
            RawKind::Null(raw) => {
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
            RawKind::Number(raw) => {
                doc.strings.get(&raw.string).fmt(f)?;
            }
            RawKind::String(raw) => {
                let string = doc.strings.get(&raw.string);

                match raw.kind {
                    StringKind::Bare => {
                        string.fmt(f)?;
                    }
                    StringKind::DoubleQuoted => {
                        write!(f, "\"{string}\"")?;
                    }
                    StringKind::SingleQuoted => {
                        write!(f, "'{string}'")?;
                    }
                }
            }
            RawKind::Table(table) => {
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

/// The kind of a YAML value.
#[derive(Debug, Clone)]
pub(crate) enum RawKind {
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
