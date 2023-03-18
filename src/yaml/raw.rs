use std::fmt::{self, Write};

use bstr::ByteSlice;

use crate::yaml::data::{Data, StringId, ValueId};
use crate::yaml::serde::RawNumberHint;
use crate::yaml::NullKind;

/// Construct a raw kind associated with booleans.
pub(crate) fn new_bool(data: &mut Data, value: bool) -> RawKind {
    const TRUE: &[u8] = b"true";
    const FALSE: &[u8] = b"false";

    let string = data.insert_str(if value { TRUE } else { FALSE });
    RawKind::String(RawString::new(RawStringKind::Bare, string))
}

/// Construct a raw kind associated with a string.
pub(crate) fn new_string<S>(data: &mut Data, string: S) -> RawKind
where
    S: AsRef<str>,
{
    let kind = RawStringKind::detect(string.as_ref());
    let string = data.insert_str(string.as_ref());
    RawKind::String(RawString::new(kind, string))
}

#[derive(Debug, Clone)]
pub(crate) struct Layout {
    pub(crate) indent: StringId,
}

#[derive(Debug, Clone)]
pub(crate) struct Raw {
    pub(crate) kind: RawKind,
    pub(crate) layout: Layout,
}

impl Raw {
    pub(crate) fn new(kind: RawKind, indent: StringId) -> Self {
        Self {
            kind,
            layout: Layout { indent },
        }
    }

    pub(crate) fn display(&self, data: &Data, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            RawKind::Null(raw) => {
                raw.display(f)?;
            }
            RawKind::Number(raw) => {
                raw.display(data, f)?;
            }
            RawKind::String(raw) => {
                raw.display(data, f)?;
            }
            RawKind::Mapping(raw) => {
                raw.display(data, f)?;
            }
            RawKind::Sequence(raw) => {
                raw.display(data, f)?;
            }
        }

        Ok(())
    }
}

/// A raw value.
#[derive(Debug, Clone)]
pub(crate) enum RawKind {
    /// A null value.
    Null(NullKind),
    /// A single number.
    Number(RawNumber),
    /// A string.
    String(RawString),
    /// A mapping.
    Mapping(RawMapping),
    /// A sequence.
    Sequence(RawSequence),
}

/// A YAML number.
#[derive(Debug, Clone)]
pub(crate) struct RawNumber {
    pub(crate) string: StringId,
    #[cfg_attr(not(feature = "serde"), allow(unused))]
    pub(crate) hint: RawNumberHint,
}

impl RawNumber {
    /// A simple number.
    pub(crate) fn new(string: StringId, hint: RawNumberHint) -> Self {
        Self { string, hint }
    }

    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        let number = data.str(&self.string);
        write!(f, "{number}")
    }
}

/// The kind of string value.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) enum RawStringKind {
    /// A bare string without quotes, such as `hello-world`.
    Bare,
    /// A single-quoted string.
    SingleQuoted,
    /// A double-quoted string.
    DoubleQuoted,
    /// An escaped string, where the string id points to the original string.
    Original(StringId),
}

impl RawStringKind {
    /// Detect the appropriate kind to use for the given string.
    pub(crate) fn detect(string: &str) -> RawStringKind {
        let mut kind = RawStringKind::Bare;

        for c in string.chars() {
            match c {
                '\'' => {
                    return RawStringKind::DoubleQuoted;
                }
                ':' => {
                    kind = RawStringKind::SingleQuoted;
                }
                b if b.is_control() => {
                    return RawStringKind::DoubleQuoted;
                }
                _ => {}
            }
        }

        kind
    }
}

/// A YAML string.
#[derive(Debug, Clone)]
pub(crate) struct RawString {
    /// The kind of the string.
    pub(crate) kind: RawStringKind,
    /// The content of the string.
    pub(crate) string: StringId,
}

impl RawString {
    /// A simple number.
    pub(crate) fn new(kind: RawStringKind, string: StringId) -> Self {
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
            RawStringKind::Original(original) => {
                let string = data.str(original);
                write!(f, "{string}")?;
            }
            RawStringKind::Bare => {
                let string = data.str(&self.string);
                write!(f, "{string}")?;
            }
            RawStringKind::DoubleQuoted => {
                let string = data.str(&self.string);
                escape_double_quoted(string, f)?;
            }
            RawStringKind::SingleQuoted => {
                let string = data.str(&self.string);
                escape_single_quoted(string, f)?;
            }
        }

        Ok(())
    }
}

/// The kind of a raw sequence.
#[derive(Debug, Clone)]
pub(crate) enum RawSequenceKind {
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

/// An element in a YAML sequence.
#[derive(Debug, Clone)]
pub(crate) struct RawSequenceItem {
    pub(crate) prefix: Option<StringId>,
    pub(crate) separator: StringId,
    pub(crate) value: ValueId,
}

/// A YAML sequence.
#[derive(Debug, Clone)]
pub(crate) struct RawSequence {
    /// The kind of a raw sequence.
    pub(crate) kind: RawSequenceKind,
    /// Items in the sequence.
    pub(crate) items: Vec<RawSequenceItem>,
}

impl RawSequence {
    /// Display the sequence.
    pub(crate) fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        if let RawSequenceKind::Inline { .. } = &self.kind {
            write!(f, "[")?;
        }

        let mut it = self.items.iter().peekable();

        while let Some(item) = it.next() {
            if let Some(prefix) = &item.prefix {
                let prefix = data.str(prefix);
                write!(f, "{prefix}")?;
            }

            if let RawSequenceKind::Mapping = self.kind {
                write!(f, "-")?;
            }

            let separator = data.str(&item.separator);
            write!(f, "{separator}")?;

            data.raw(item.value).display(data, f)?;

            if it.peek().is_some() {
                if let RawSequenceKind::Inline { .. } = self.kind {
                    write!(f, ",")?;
                }
            }
        }

        if let RawSequenceKind::Inline { trailing, suffix } = &self.kind {
            if *trailing {
                write!(f, ",")?;
            }

            write!(f, "{}]", data.str(suffix))?;
        }

        Ok(())
    }
}

/// An element in a YAML mapping.
#[derive(Debug, Clone)]
pub(crate) struct RawMappingItem {
    pub(crate) prefix: Option<StringId>,
    pub(crate) key: RawString,
    pub(crate) separator: StringId,
    pub(crate) value: ValueId,
}

/// The kind of a raw mapping.
#[derive(Debug, Clone)]
pub(crate) enum RawMappingKind {
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
pub(crate) struct RawMapping {
    pub(crate) kind: RawMappingKind,
    pub(crate) items: Vec<RawMappingItem>,
}

impl RawMapping {
    /// Display the mapping.
    pub(crate) fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        if let RawMappingKind::Inline { .. } = &self.kind {
            write!(f, "{{")?;
        }

        let mut it = self.items.iter().peekable();

        while let Some(item) = it.next() {
            if let Some(prefix) = &item.prefix {
                let prefix = data.str(prefix);
                write!(f, "{prefix}")?;
            }

            let key = data.str(&item.key.string);
            let separator = data.str(&item.separator);
            write!(f, "{key}:{separator}")?;

            data.raw(item.value).display(data, f)?;

            if it.peek().is_some() {
                if let RawMappingKind::Inline { .. } = &self.kind {
                    write!(f, ",")?;
                }
            }
        }

        if let RawMappingKind::Inline { trailing, suffix } = &self.kind {
            if *trailing {
                write!(f, ",")?;
            }

            let suffix = data.str(suffix);
            write!(f, "{suffix}}}")?;
        }

        Ok(())
    }
}
