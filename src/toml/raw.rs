use std::fmt;
use std::io;

use crate::serde_hint::RawNumberHint;
use crate::toml::data::{Data, Id, StringId};

/// Newline character used in TOML.
pub(crate) const NEWLINE: u8 = b'\n';
/// Space character used in TOML.
pub(crate) const SPACE: u8 = b' ';

#[derive(Debug, Clone)]
pub(crate) enum Raw {
    /// An empty value.
    Empty,
    /// A single number.
    Number(Number),
    /// A string.
    String(String),
    /// A raw table.
    Table(Table),
    /// A single item inside of a table.
    TableItem(TableItem),
}

impl Raw {
    pub(crate) fn display(&self, data: &Data, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            _ => {}
        }

        Ok(())
    }

    pub(crate) fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        match self {
            _ => {}
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct Layout {
    /// Reference to the indentation just preceeding the current value.
    pub(crate) prefix: StringId,
    /// Reference to the parent of a value.
    #[allow(unused)]
    pub(crate) parent: Option<Id>,
}

/// A TOML number.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct Number {
    pub(crate) string: StringId,
    #[cfg_attr(not(feature = "serde"), allow(unused))]
    pub(crate) hint: RawNumberHint,
}

impl Number {
    /// A simple number.
    pub(crate) fn new(string: StringId, hint: RawNumberHint) -> Self {
        Self { string, hint }
    }

    #[inline]
    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", data.str(self.string))
    }

    #[inline]
    pub(crate) fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        o.write_all(data.str(self.string))
    }
}

/// A TOML string.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct String {
    /// The kind of the string.
    pub(crate) kind: RawStringKind,
    /// The content of the string.
    pub(crate) string: StringId,
}

/// The kind of string value.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(tag = "kind"))]
#[non_exhaustive]
pub(crate) enum RawStringKind {
    /// A bare string without quotes, such as `hello-world`.
    Bare,
}

/// A table.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct Table {
    pub(crate) items: Vec<Id>,
}

/// A table element.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct TableItem {
    pub(crate) key: String,
    pub(crate) sep: StringId,
    pub(crate) value: Id,
}

impl TableItem {
    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", data.prefix(self.value))?;
        data.raw(self.value).display(data, f)?;
        Ok(())
    }

    fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        o.write_all(data.prefix(self.value))?;
        data.raw(self.value).write_to(data, o)?;
        Ok(())
    }
}
