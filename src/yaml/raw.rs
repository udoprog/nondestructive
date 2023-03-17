use std::fmt::{self, Write};

use bstr::ByteSlice;

use crate::strings::{StringId, Strings};
use crate::yaml::{NullKind, Separator, StringKind};

/// Construct a raw kind associated with booleans.
pub(crate) fn new_bool(strings: &mut Strings, value: bool) -> RawKind {
    const TRUE: &[u8] = b"true";
    const FALSE: &[u8] = b"false";

    let string = strings.insert(if value { TRUE } else { FALSE });
    RawKind::String(RawString::new(StringKind::Bare, string))
}

/// Construct a raw kind associated with a string.
pub(crate) fn new_string<S>(strings: &mut Strings, string: S) -> RawKind
where
    S: AsRef<str>,
{
    let kind = StringKind::detect(string.as_ref());
    let string = strings.insert(string.as_ref());
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

    pub(crate) fn display(&self, strings: &Strings, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            RawKind::Null(raw) => {
                raw.display(f)?;
            }
            RawKind::Number(raw) => {
                raw.display(strings, f)?;
            }
            RawKind::String(raw) => {
                raw.display(strings, f)?;
            }
            RawKind::Table(raw) => {
                raw.display(strings, f)?;
            }
            RawKind::List(raw) => {
                raw.display(strings, f)?;
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
    /// A table.
    Table(RawTable),
    /// A list.
    List(RawList),
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

    fn display(&self, strings: &Strings, f: &mut fmt::Formatter) -> fmt::Result {
        let number = strings.get(&self.string);
        write!(f, "{number}")
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

    fn display(&self, strings: &Strings, f: &mut fmt::Formatter) -> fmt::Result {
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

        let string = strings.get(&self.string);

        match &self.kind {
            StringKind::Bare => {
                write!(f, "{string}")?;
            }
            StringKind::DoubleQuoted => {
                escape_double_quoted(string, f)?;
            }
            StringKind::SingleQuoted => {
                escape_single_quoted(string, f)?;
            }
        }

        Ok(())
    }
}

/// The kind of a raw list.
#[derive(Debug, Clone)]
pub(crate) enum RawListKind {
    /// An expanded tabular YAML list.
    ///
    /// ```yaml
    /// - one
    /// - two
    /// - three
    /// ```
    Table,
    /// A compact inline YAML list.
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

/// An element in a YAML list.
#[derive(Debug, Clone)]
pub(crate) struct RawListItem {
    pub(crate) prefix: Option<StringId>,
    pub(crate) separator: StringId,
    pub(crate) value: Box<Raw>,
}

/// A YAML list.
#[derive(Debug, Clone)]
pub(crate) struct RawList {
    /// The kind of a raw list.
    pub(crate) kind: RawListKind,
    /// Items in the list.
    pub(crate) items: Vec<RawListItem>,
}

impl RawList {
    /// Push a value on the list.
    pub(crate) fn push(
        &mut self,
        strings: &mut Strings,
        layout: &Layout,
        separator: Separator,
        value: RawKind,
    ) {
        let separator = match separator {
            Separator::Auto => match self.items.last() {
                Some(last) => last.separator,
                None => strings.insert(" "),
            },
            Separator::Custom(separator) => strings.insert(separator),
        };

        let prefix = (!self.items.is_empty()).then_some(layout.indent);

        self.items.push(RawListItem {
            prefix,
            separator,
            value: Box::new(Raw::new(value, layout.indent)),
        });
    }

    /// Display the list.
    pub(crate) fn display(&self, strings: &Strings, f: &mut fmt::Formatter) -> fmt::Result {
        if let RawListKind::Inline { .. } = &self.kind {
            write!(f, "[")?;
        }

        let mut it = self.items.iter().peekable();

        while let Some(item) = it.next() {
            if let Some(prefix) = &item.prefix {
                let prefix = strings.get(prefix);
                write!(f, "{prefix}")?;
            }

            if let RawListKind::Table = self.kind {
                write!(f, "-")?;
            }

            let separator = strings.get(&item.separator);
            write!(f, "{separator}")?;

            item.value.display(strings, f)?;

            if it.peek().is_some() {
                if let RawListKind::Inline { .. } = self.kind {
                    write!(f, ",")?;
                }
            }
        }

        if let RawListKind::Inline { trailing, suffix } = &self.kind {
            if *trailing {
                write!(f, ",")?;
            }

            write!(f, "{}]", strings.get(suffix))?;
        }

        Ok(())
    }
}

/// An element in a YAML table.
#[derive(Debug, Clone)]
pub(crate) struct RawTableItem {
    pub(crate) prefix: Option<StringId>,
    pub(crate) key: RawString,
    pub(crate) separator: StringId,
    pub(crate) value: Box<Raw>,
}

/// The kind of a raw table.
#[derive(Debug, Clone)]
pub(crate) enum RawTableKind {
    /// An expanded tabular YAML table.
    ///
    /// ```yaml
    /// one: 1
    /// two: 2
    /// ```
    Table,
    /// A compact inline YAML table.
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

/// A YAML table.
#[derive(Debug, Clone)]
pub(crate) struct RawTable {
    pub(crate) kind: RawTableKind,
    pub(crate) items: Vec<RawTableItem>,
}

impl RawTable {
    /// Insert a value into the table.
    pub(crate) fn insert(
        &mut self,
        strings: &mut Strings,
        layout: &Layout,
        key: &str,
        separator: Separator<'_>,
        value: RawKind,
    ) -> usize {
        let key = strings.insert(key);

        if let Some(index) = self.items.iter_mut().position(|c| c.key.string == key) {
            let item = &mut self.items[index];
            item.value.kind = value;
            return index;
        }

        let key = RawString::new(StringKind::Bare, key);

        let separator = match separator {
            Separator::Auto => match self.items.last() {
                Some(last) => last.separator,
                None => strings.insert(" "),
            },
            Separator::Custom(separator) => strings.insert(separator),
        };

        let prefix = (!self.items.is_empty()).then_some(layout.indent);

        let len = self.items.len();
        self.items.push(RawTableItem {
            prefix,
            key,
            separator,
            value: Box::new(Raw::new(value, layout.indent)),
        });
        len
    }

    /// Display the table.
    pub(crate) fn display(&self, strings: &Strings, f: &mut fmt::Formatter) -> fmt::Result {
        if let RawTableKind::Inline { .. } = &self.kind {
            write!(f, "{{")?;
        }

        let mut it = self.items.iter().peekable();

        while let Some(item) = it.next() {
            if let Some(prefix) = &item.prefix {
                let prefix = strings.get(prefix);
                write!(f, "{prefix}")?;
            }

            let key = strings.get(&item.key.string);
            let separator = strings.get(&item.separator);
            write!(f, "{key}:{separator}")?;

            item.value.display(strings, f)?;

            if it.peek().is_some() {
                if let RawTableKind::Inline { .. } = &self.kind {
                    write!(f, ",")?;
                }
            }
        }

        if let RawTableKind::Inline { trailing, suffix } = &self.kind {
            if *trailing {
                write!(f, ",")?;
            }

            let suffix = strings.get(suffix);
            write!(f, "{suffix}}}")?;
        }

        Ok(())
    }
}
