use std::fmt::{self, Write};

use bstr::ByteSlice;

use crate::yaml::data::{Data, StringId, ValueId};
use crate::yaml::serde::RawNumberHint;
use crate::yaml::Null;

/// Newline character used in YAML.
pub(crate) const NEWLINE: u8 = b'\n';
/// Space character used in YAML.
pub(crate) const SPACE: u8 = b' ';

/// Construct a raw kind associated with booleans.
pub(crate) fn new_bool(value: bool) -> Raw {
    Raw::Boolean(value)
}

/// Construct a raw kind associated with a string.
pub(crate) fn new_string<S>(data: &mut Data, string: S) -> Raw
where
    S: AsRef<str>,
{
    let kind = StringKind::detect(string.as_ref());
    let string = data.insert_str(string.as_ref());
    Raw::String(String::new(kind, string))
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    /// Reference to the indentation just preceeding the current value.
    pub(crate) prefix: StringId,
    /// Reference to the parent of a value.
    #[allow(unused)]
    pub(crate) parent: Option<ValueId>,
}

/// A raw value.
#[derive(Debug, Clone)]
pub(crate) enum Raw {
    /// A null value.
    Null(Null),
    /// A boolean value.
    Boolean(bool),
    /// A single number.
    Number(Number),
    /// A string.
    String(String),
    /// A mapping.
    Mapping(Mapping),
    /// A single item inside of a mapping.
    MappingItem(MappingItem),
    /// A sequence.
    Sequence(Sequence),
    /// A single item inside of a sequence.
    SequenceItem(SequenceItem),
}

impl Raw {
    pub(crate) fn display(&self, data: &Data, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Raw::Null(raw) => {
                raw.display(f)?;
            }
            Raw::Boolean(raw) => {
                if *raw {
                    write!(f, "true")?;
                } else {
                    write!(f, "false")?;
                }
            }
            Raw::Number(raw) => {
                raw.display(data, f)?;
            }
            Raw::String(raw) => {
                raw.display(data, f)?;
            }
            Raw::Mapping(raw) => {
                raw.display(data, f)?;
            }
            Raw::MappingItem(raw) => {
                raw.display(data, f)?;
            }
            Raw::Sequence(raw) => {
                raw.display(data, f)?;
            }
            Raw::SequenceItem(raw) => {
                raw.display(data, f)?;
            }
        }

        Ok(())
    }
}

macro_rules! from {
    ($ident:ident) => {
        impl From<$ident> for Raw {
            #[inline]
            fn from(value: $ident) -> Self {
                Raw::$ident(value)
            }
        }
    };
}

from!(Mapping);
from!(MappingItem);
from!(Sequence);
from!(SequenceItem);

/// A YAML number.
#[derive(Debug, Clone)]
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
}

/// The kind of string value.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) enum StringKind {
    /// A bare string without quotes, such as `hello-world`.
    Bare,
    /// A single-quoted string.
    SingleQuoted,
    /// A double-quoted string.
    DoubleQuoted,
    /// An escaped string, where the string id points to the original string.
    Original(StringId),
    /// A multiline string.
    Multiline(u8, StringId),
}

impl StringKind {
    /// Detect the appropriate kind to use for the given string.
    pub(crate) fn detect(string: &str) -> StringKind {
        if matches!(string, "true" | "false" | "null") {
            return StringKind::SingleQuoted;
        }

        let mut kind = StringKind::Bare;

        for c in string.chars() {
            match c {
                '\'' => {
                    return StringKind::DoubleQuoted;
                }
                ':' => {
                    kind = StringKind::SingleQuoted;
                }
                b if b.is_control() => {
                    return StringKind::DoubleQuoted;
                }
                _ => {}
            }
        }

        kind
    }
}

/// A YAML string.
#[derive(Debug, Clone)]
pub(crate) struct String {
    /// The kind of the string.
    pub(crate) kind: StringKind,
    /// The content of the string.
    pub(crate) string: StringId,
}

impl String {
    /// A simple number.
    pub(crate) fn new(kind: StringKind, string: StringId) -> Self {
        Self { kind, string }
    }

    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        /// Single-quoted escape sequences:
        /// <https://yaml.org/spec/1.2.2/#escaped-characters>.
        fn escape_single_quoted(
            string: &bstr::BStr,
            f: &mut fmt::Formatter,
        ) -> Result<(), fmt::Error> {
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
        fn escape_double_quoted(
            string: &bstr::BStr,
            f: &mut fmt::Formatter,
        ) -> Result<(), fmt::Error> {
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

        match &self.kind {
            StringKind::Bare => {
                let string = data.str(self.string);
                write!(f, "{string}")?;
            }
            StringKind::DoubleQuoted => {
                let string = data.str(self.string);
                escape_double_quoted(string, f)?;
            }
            StringKind::SingleQuoted => {
                let string = data.str(self.string);
                escape_single_quoted(string, f)?;
            }
            StringKind::Original(original) => {
                let string = data.str(*original);
                write!(f, "{string}")?;
            }
            StringKind::Multiline(prefix, original) => {
                let string = data.str(*original);
                write!(f, "{}{string}", char::from(*prefix))?;
            }
        }

        Ok(())
    }
}

/// The kind of a raw sequence.
#[derive(Debug, Clone)]
pub(crate) enum SequenceKind {
    /// An expanded tabular YAML sequence.
    ///
    /// ```yaml
    /// - one
    /// - two
    /// - three
    /// ```
    Mapping,
    /// A compact inline YAML sequence.
    ///
    /// ```yaml
    /// [one two three]
    /// ```
    Inline {
        /// Trailing `,` separator.
        trailing: bool,
        /// The inner suffix, before the trailing `]`.
        suffix: StringId,
    },
}

/// A YAML sequence.
#[derive(Debug, Clone)]
pub(crate) struct Sequence {
    /// Indentation used for this sequence.
    pub(crate) indent: usize,
    /// The kind of a raw sequence.
    pub(crate) kind: SequenceKind,
    /// Items in the sequence.
    pub(crate) items: Vec<ValueId>,
}

impl Sequence {
    /// Display the sequence.
    pub(crate) fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        if let SequenceKind::Inline { .. } = &self.kind {
            write!(f, "[")?;
        }

        let mut it = self.items.iter().peekable();

        while let Some(item) = it.next() {
            write!(f, "{}", data.prefix(*item))?;

            if let SequenceKind::Mapping = self.kind {
                write!(f, "-")?;
            }

            data.sequence_item(*item).display(data, f)?;

            if it.peek().is_some() {
                if let SequenceKind::Inline { .. } = self.kind {
                    write!(f, ",")?;
                }
            }
        }

        if let SequenceKind::Inline { trailing, suffix } = &self.kind {
            if *trailing {
                write!(f, ",")?;
            }

            write!(f, "{}]", data.str(*suffix))?;
        }

        Ok(())
    }
}

/// An element in a YAML sequence.
#[derive(Debug, Clone)]
pub(crate) struct SequenceItem {
    pub(crate) value: ValueId,
}

impl SequenceItem {
    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", data.prefix(self.value))?;
        data.raw(self.value).display(data, f)?;
        Ok(())
    }
}

/// The kind of a raw mapping.
#[derive(Debug, Clone)]
pub(crate) enum MappingKind {
    /// An expanded tabular YAML mapping.
    ///
    /// ```yaml
    /// one: 1
    /// two: 2
    /// ```
    Mapping,
    /// A compact inline YAML mapping.
    ///
    /// ```yaml
    /// {one: 1, two: 2}
    /// ```
    Inline {
        /// Trailing `,` separator.
        trailing: bool,
        /// The inner suffix, before the trailing `]`.
        suffix: StringId,
    },
}

/// A YAML mapping.
#[derive(Debug, Clone)]
pub(crate) struct Mapping {
    pub(crate) indent: usize,
    pub(crate) kind: MappingKind,
    pub(crate) items: Vec<ValueId>,
}

impl Mapping {
    /// Display the mapping.
    pub(crate) fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        if let MappingKind::Inline { .. } = &self.kind {
            write!(f, "{{")?;
        }

        let mut it = self.items.iter().peekable();

        while let Some(id) = it.next() {
            let item = data.mapping_item(*id);
            write!(f, "{}", data.prefix(*id))?;
            item.display(data, f)?;

            if it.peek().is_some() {
                if let MappingKind::Inline { .. } = &self.kind {
                    write!(f, ",")?;
                }
            }
        }

        if let MappingKind::Inline { trailing, suffix } = &self.kind {
            if *trailing {
                write!(f, ",")?;
            }

            let suffix = data.str(*suffix);
            write!(f, "{suffix}}}")?;
        }

        Ok(())
    }
}

/// An element in a YAML mapping.
#[derive(Debug, Clone)]
pub(crate) struct MappingItem {
    pub(crate) key: String,
    pub(crate) value: ValueId,
}

impl MappingItem {
    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        let key = data.str(self.key.string);
        write!(f, "{key}:")?;
        write!(f, "{}", data.prefix(self.value))?;
        data.raw(self.value).display(data, f)?;
        Ok(())
    }
}
