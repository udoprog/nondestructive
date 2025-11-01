use core::fmt::{self, Write};
use core::mem;

use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::io;

use bstr::{BStr, ByteSlice};
#[cfg(feature = "serde-edits")]
use serde::{Deserialize, Serialize};

use crate::yaml::data::{Data, Id, StringId};
use crate::yaml::serde_hint::RawNumberHint;
use crate::yaml::{Block, Chomp, StringKind};

/// Newline character used in YAML.
pub(crate) const NEWLINE: u8 = b'\n';
/// Space character used in YAML.
pub(crate) const SPACE: u8 = b' ';

/// Get the indentation for the given string.
pub(crate) fn indent(string: &[u8]) -> &[u8] {
    match memchr::memrchr(NEWLINE, string) {
        Some(n) => n
            .checked_add(1)
            .and_then(|n| string.get(n..))
            .unwrap_or_default(),
        None => string,
    }
}

/// Count indentation level for the given string.
pub(crate) fn count_indent(string: &[u8]) -> usize {
    indent(string).chars().count()
}

/// Construct a raw kind associated with booleans.
pub(crate) fn new_bool(data: &mut Data, value: bool) -> Raw {
    let string = data.insert_str(if value { "true" } else { "false" });
    Raw::Boolean(Boolean::new(value, string))
}

/// Construct a raw kind associated with a string.
pub(crate) fn new_string<S>(data: &mut Data, string: S) -> Raw
where
    S: AsRef<str>,
{
    let kind = RawStringKind::detect(string.as_ref());
    let string = data.insert_str(string.as_ref());
    Raw::String(String::new(kind, string, string))
}

/// Construct an indentation prefix.
pub(crate) fn make_indent(data: &mut Data, id: Id) -> (usize, StringId) {
    let parent = data.layout(id).parent;

    let Some(container) = parent.and_then(|id| data.layout(id).parent) else {
        let prefix = data.layout(id).prefix;
        let indent = self::count_indent(data.str(prefix));
        return (indent, prefix);
    };

    let (raw, layout) = data.pair(container);

    let indent = match raw {
        Raw::Mapping(raw) => raw.indent,
        Raw::Sequence(raw) => raw.indent,
        _ => {
            let prefix = data.layout(id).prefix;
            let indent = self::count_indent(data.str(prefix));
            return (indent, prefix);
        }
    };

    let is_sequence_mapping = matches!(
        raw,
        Raw::Sequence(Sequence {
            kind: SequenceKind::Mapping,
            ..
        })
    );

    if is_sequence_mapping && matches!(parent.map(|id| data.raw(id)), Some(Raw::SequenceItem(..))) {
        // If we are an immediate child of a sequence item, then we want to be
        // nested at the same level as the item.
        //
        // This allows for a more natural representation of sequences, like
        // these:
        //
        // ```yaml
        // - one
        // - two: 2
        //   three: 3
        // ```
        //
        // Use the sequence's base indentation plus the dash and space (2 chars)
        let indent = indent.saturating_add(2);
        return (indent, data.insert_str(" "));
    }

    let indent = indent.saturating_add(2);
    // Take some pains to preserve the existing suffix, synthesize extra spaces characters where needed.
    let mut existing = self::indent(data.str(layout.prefix)).chars();

    let mut prefix = Vec::new();
    prefix.push(NEWLINE);

    for _ in 0..indent {
        if let Some(c) = existing.next() {
            prefix.extend(c.encode_utf8(&mut [0; 4]).as_bytes());
        } else {
            prefix.push(SPACE);
        }
    }

    (indent, data.insert_str(prefix))
}

/// Construct a raw kind associated with a string with a custom string kind.
pub(crate) fn new_string_with<S>(data: &mut Data, string: S, kind: StringKind) -> Raw
where
    S: AsRef<str>,
{
    let kind = match kind {
        StringKind::Bare => RawStringKind::Bare,
        StringKind::Single => RawStringKind::Single,
        StringKind::Double => RawStringKind::Double,
    };

    let string = data.insert_str(string.as_ref());
    Raw::String(String::new(kind, string, string))
}

/// Construct a block with the given configuration.
pub(crate) fn new_block<I>(data: &mut Data, id: Id, iter: I, block: Block) -> Raw
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let (indent, prefix) = match data.raw(id) {
        Raw::Mapping(raw) => (raw.indent.wrapping_add(2), BStr::new(b"")),
        Raw::Sequence(raw) => (raw.indent.wrapping_add(2), BStr::new(b"")),
        _ => {
            let prefix = data.str(data.layout(id).prefix);

            let n = prefix.rfind([NEWLINE]).map_or(0, |i| i.wrapping_add(1));

            (2, &prefix[n..])
        }
    };

    let mut original = Vec::new();
    let mut out = Vec::new();

    let (mark, join, chomp) = match block {
        Block::Literal(chomp) => (b'|', b'\n', chomp),
        Block::Folded(chomp) => (b'>', b' ', chomp),
    };

    original.push(mark);
    original.extend(chomp.as_byte());

    let mut it = iter.into_iter().peekable();

    while let Some(part) = it.next() {
        original.push(NEWLINE);
        original.extend_from_slice(prefix.as_bytes());
        original.resize(original.len() + indent, SPACE);
        original.extend(part.as_ref().as_bytes());
        out.extend(part.as_ref().as_bytes());

        if it.peek().is_some() {
            out.push(join);
        }
    }

    if let Chomp::Clip | Chomp::Keep = chomp {
        out.push(NEWLINE);
    }

    let original = data.insert_str(&original);
    let string = data.insert_str(out);

    Raw::String(self::String::new(RawStringKind::Original, string, original))
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

/// A raw value.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(tag = "kind", content = "value"))]
pub(crate) enum Raw {
    /// A null value.
    Null(Null),
    /// A boolean value.
    Boolean(Boolean),
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
    pub(crate) fn display(
        &self,
        data: &Data,
        f: &mut fmt::Formatter<'_>,
        prefix: Option<Id>,
    ) -> fmt::Result {
        match self {
            Raw::Null(raw) => {
                raw.display(data, f, prefix)?;
            }
            Raw::Boolean(raw) => {
                if let Some(id) = prefix {
                    write!(f, "{}", data.prefix(id))?;
                }

                raw.display(data, f)?;
            }
            Raw::Number(raw) => {
                if let Some(id) = prefix {
                    write!(f, "{}", data.prefix(id))?;
                }

                raw.display(data, f)?;
            }
            Raw::String(raw) => {
                if let Some(id) = prefix {
                    write!(f, "{}", data.prefix(id))?;
                }

                raw.display(data, f)?;
            }
            Raw::Mapping(raw) => {
                raw.display(data, f, prefix)?;
            }
            Raw::MappingItem(raw) => {
                if let Some(id) = prefix {
                    write!(f, "{}", data.prefix(id))?;
                }

                raw.display(data, f)?;
            }
            Raw::Sequence(raw) => {
                raw.display(data, f, prefix)?;
            }
            Raw::SequenceItem(raw) => {
                if let Some(id) = prefix {
                    write!(f, "{}", data.prefix(id))?;
                }

                raw.display(data, f)?;
            }
        }

        Ok(())
    }

    #[cfg(feature = "std")]
    pub(crate) fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        match self {
            Raw::Null(raw) => {
                raw.write_to(data, o)?;
            }
            Raw::Boolean(raw) => {
                raw.write_to(data, o)?;
            }
            Raw::Number(raw) => {
                raw.write_to(data, o)?;
            }
            Raw::String(raw) => {
                raw.write_to(data, o)?;
            }
            Raw::Mapping(raw) => {
                raw.write_to(data, o)?;
            }
            Raw::MappingItem(raw) => {
                raw.write_to(data, o)?;
            }
            Raw::Sequence(raw) => {
                raw.write_to(data, o)?;
            }
            Raw::SequenceItem(raw) => {
                raw.write_to(data, o)?;
            }
        }

        Ok(())
    }

    pub(crate) fn is_tabular(&self) -> bool {
        matches!(
            self,
            Self::Sequence(Sequence {
                kind: SequenceKind::Mapping,
                ..
            }) | Self::Mapping(Mapping {
                kind: MappingKind::Mapping,
                ..
            })
        )
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

/// A YAML null.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) enum Null {
    /// A keyword `null` value.
    Keyword(StringId),
    /// A tilde `~` null value.
    Tilde,
    /// An empty null value.
    Empty,
}

impl Null {
    pub(crate) fn display(
        &self,
        data: &Data,
        f: &mut fmt::Formatter<'_>,
        prefix: Option<Id>,
    ) -> fmt::Result {
        match self {
            Null::Keyword(string) => {
                if let Some(id) = prefix {
                    write!(f, "{}", data.prefix(id))?;
                }

                write!(f, "{}", data.str(*string))?;
            }
            Null::Tilde => {
                if let Some(id) = prefix {
                    write!(f, "{}", data.prefix(id))?;
                }

                write!(f, "~")?;
            }
            Null::Empty => {
                // empty values count as null.
            }
        }

        Ok(())
    }

    #[cfg(feature = "std")]
    pub(crate) fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        match self {
            Null::Keyword(string) => {
                o.write_all(data.str(*string))?;
            }
            Null::Tilde => {
                write!(o, "~")?;
            }
            Null::Empty => {
                // empty values count as null.
            }
        }

        Ok(())
    }
}

/// A YAML boolean.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct Boolean {
    pub(crate) value: bool,
    pub(crate) string: StringId,
}

impl Boolean {
    pub(crate) fn new(value: bool, string: StringId) -> Self {
        Self { value, string }
    }

    #[inline]
    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", data.str(self.string))
    }

    #[cfg(feature = "std")]
    #[inline]
    pub(crate) fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        o.write_all(data.str(self.string))
    }
}

/// A YAML number.
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

    #[cfg(feature = "std")]
    #[inline]
    pub(crate) fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        o.write_all(data.str(self.string))
    }
}

/// The kind of string value.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(tag = "kind"))]
#[non_exhaustive]
pub(crate) enum RawStringKind {
    /// A bare string without quotes, such as `hello-world`.
    Bare,
    /// A single-quoted string.
    Single,
    /// A double-quoted string.
    Double,
    /// An escaped string, where the string id points to the original string.
    Original,
    /// A multiline string.
    Multiline { prefix: StringId },
}

impl RawStringKind {
    /// Detect the appropriate kind to use for the given string.
    pub(crate) fn detect(string: &str) -> RawStringKind {
        if matches!(string, "true" | "false" | "null") {
            return RawStringKind::Single;
        }

        let mut kind = RawStringKind::Bare;
        let mut first = true;

        for c in string.chars() {
            let first = mem::take(&mut first);

            match c {
                '0'..='9' if first => {
                    kind = RawStringKind::Single;
                }
                '\'' => {
                    return RawStringKind::Double;
                }
                ':' => {
                    kind = RawStringKind::Single;
                }
                b if b.is_control() => {
                    return RawStringKind::Double;
                }
                _ => {}
            }
        }

        kind
    }
}

/// A YAML string.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct String {
    /// The kind of the string.
    pub(crate) kind: RawStringKind,
    /// The content of the string.
    pub(crate) id: StringId,
    /// The original contents of the string.
    pub(crate) original: StringId,
}

impl String {
    /// A simple number.
    pub(crate) fn new(kind: RawStringKind, id: StringId, original: StringId) -> Self {
        Self { kind, id, original }
    }

    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        /// Single-quoted escape sequences:
        /// <https://yaml.org/spec/1.2.2/#escaped-characters>.
        fn escape_single_quoted(mut string: &bstr::BStr, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_char('\'')?;

            loop {
                let Some(n) = memchr::memchr(b'\'', string) else {
                    write!(f, "{string}")?;
                    break;
                };

                write!(f, "{}", &string[..n])?;
                f.write_str("''")?;
                string = &string[n.saturating_add(1)..];
            }

            f.write_char('\'')?;
            Ok(())
        }

        /// Double-quoted escape sequences:
        /// <https://yaml.org/spec/1.2.2/#escaped-characters>.
        fn escape_double_quoted(string: &bstr::BStr, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_char('"')?;

            let mut start = 0;

            for (index, end, c) in string.char_indices() {
                let esc = match c {
                    '\u{00}' => "\\0",
                    '\u{07}' => "\\a",
                    '\u{08}' => "\\b",
                    '\u{09}' => "\\t",
                    '\n' => "\\n",
                    '\u{0b}' => "\\v",
                    '\u{0c}' => "\\f",
                    '\r' => "\\r",
                    '\u{1b}' => "\\e",
                    '\"' => "\\\"",
                    c if c.is_ascii_control() => {
                        write!(f, "{}\\x{:02x}", &string[start..index], c as u8)?;
                        start = end;
                        continue;
                    }
                    _ => {
                        continue;
                    }
                };

                write!(f, "{}{esc}", &string[start..index])?;
                start = end;
            }

            write!(f, "{}", &string[start..])?;
            f.write_char('"')?;
            Ok(())
        }

        match &self.kind {
            RawStringKind::Bare => {
                let string = data.str(self.id);
                write!(f, "{string}")?;
            }
            RawStringKind::Double => {
                let string = data.str(self.id);
                escape_double_quoted(string, f)?;
            }
            RawStringKind::Single => {
                let string = data.str(self.id);
                escape_single_quoted(string, f)?;
            }
            RawStringKind::Original => {
                let string = data.str(self.original);
                write!(f, "{string}")?;
            }
            RawStringKind::Multiline { prefix } => {
                let string = data.str(self.original);
                write!(f, "{}{string}", data.str(*prefix))?;
            }
        }

        Ok(())
    }

    #[cfg(feature = "std")]
    fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        /// Single-quoted escape sequences:
        /// <https://yaml.org/spec/1.2.2/#escaped-characters>.
        fn escape_single_quoted<O>(mut string: &bstr::BStr, f: &mut O) -> io::Result<()>
        where
            O: ?Sized + io::Write,
        {
            f.write_all(b"\'")?;

            loop {
                let Some(index) = memchr::memchr(b'\'', string) else {
                    f.write_all(string)?;
                    break;
                };

                f.write_all(&string[..index])?;
                f.write_all(b"''")?;
                string = &string[index.saturating_add(1)..];
            }

            f.write_all(b"\'")?;
            Ok(())
        }

        /// Double-quoted escape sequences:
        /// <https://yaml.org/spec/1.2.2/#escaped-characters>.
        fn escape_double_quoted<O>(string: &bstr::BStr, o: &mut O) -> io::Result<()>
        where
            O: ?Sized + io::Write,
        {
            o.write_all(b"\"")?;
            let mut s = 0;

            for (index, b) in string.bytes().enumerate() {
                let esc = match b {
                    b'\0' => b"\\0",
                    0x07 => b"\\a",
                    0x08 => b"\\b",
                    0x09 => b"\\t",
                    b'\n' => b"\\n",
                    0x0b => b"\\v",
                    0x0c => b"\\f",
                    b'\r' => b"\\r",
                    0x1b => b"\\e",
                    b'\"' => b"\\\"",
                    c if c.is_ascii_control() => {
                        o.write_all(&string[s..index])?;
                        write!(o, "\\x{c:02x}")?;
                        s = index.saturating_add(1);
                        continue;
                    }
                    _ => {
                        continue;
                    }
                };

                o.write_all(&string[s..index])?;
                o.write_all(esc)?;
                s = index.saturating_add(1);
            }

            o.write_all(&string[s..])?;
            o.write_all(b"\"")?;
            Ok(())
        }

        match &self.kind {
            RawStringKind::Bare => {
                o.write_all(data.str(self.id))?;
            }
            RawStringKind::Double => {
                let string = data.str(self.id);
                escape_double_quoted(string, o)?;
            }
            RawStringKind::Single => {
                let string = data.str(self.id);
                escape_single_quoted(string, o)?;
            }
            RawStringKind::Original => {
                o.write_all(data.str(self.original))?;
            }
            RawStringKind::Multiline { prefix } => {
                o.write_all(data.str(*prefix))?;
                o.write_all(data.str(self.original))?;
            }
        }

        Ok(())
    }
}

/// The kind of a raw sequence.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(tag = "kind"))]
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
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct Sequence {
    /// Indentation used for this sequence.
    pub(crate) indent: usize,
    /// The kind of a raw sequence.
    pub(crate) kind: SequenceKind,
    /// Items in the sequence.
    pub(crate) items: Vec<Id>,
}

impl Sequence {
    /// Display the sequence.
    pub(crate) fn display(
        &self,
        data: &Data,
        f: &mut fmt::Formatter,
        prefix: Option<Id>,
    ) -> fmt::Result {
        if matches!(self.kind, SequenceKind::Inline { .. }) || !self.items.is_empty() {
            if let Some(id) = prefix {
                write!(f, "{}", data.prefix(id))?;
            }
        }

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

    #[cfg(feature = "std")]
    fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        if let SequenceKind::Inline { .. } = &self.kind {
            write!(o, "[")?;
        }

        let mut it = self.items.iter().peekable();

        while let Some(item) = it.next() {
            write!(o, "{}", data.prefix(*item))?;

            if let SequenceKind::Mapping = self.kind {
                write!(o, "-")?;
            }

            data.sequence_item(*item).write_to(data, o)?;

            if it.peek().is_some() {
                if let SequenceKind::Inline { .. } = self.kind {
                    write!(o, ",")?;
                }
            }
        }

        if let SequenceKind::Inline { trailing, suffix } = &self.kind {
            if *trailing {
                write!(o, ",")?;
            }

            o.write_all(data.str(*suffix))?;
            write!(o, "]")?;
        }

        Ok(())
    }
}

/// An element in a YAML sequence.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct SequenceItem {
    pub(crate) value: Id,
}

impl SequenceItem {
    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        data.raw(self.value).display(data, f, Some(self.value))?;
        Ok(())
    }

    #[cfg(feature = "std")]
    fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        o.write_all(data.prefix(self.value))?;
        data.raw(self.value).write_to(data, o)?;
        Ok(())
    }
}

/// The kind of a raw mapping.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(tag = "kind"))]
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
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct Mapping {
    /// Number of unicode characters worth of indentation in this mapping.
    pub(crate) indent: usize,
    /// The kind of the mapping.
    pub(crate) kind: MappingKind,
    /// Items inside of the mapping.
    pub(crate) items: Vec<Id>,
}

impl Mapping {
    /// Display the mapping.
    pub(crate) fn display(
        &self,
        data: &Data,
        f: &mut fmt::Formatter,
        prefix: Option<Id>,
    ) -> fmt::Result {
        if matches!(self.kind, MappingKind::Inline { .. }) || !self.items.is_empty() {
            if let Some(id) = prefix {
                write!(f, "{}", data.prefix(id))?;
            }
        }

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

    #[cfg(feature = "std")]
    fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        if let MappingKind::Inline { .. } = &self.kind {
            write!(o, "{{")?;
        }

        let mut it = self.items.iter().peekable();

        while let Some(id) = it.next() {
            o.write_all(data.prefix(*id))?;
            data.mapping_item(*id).write_to(data, o)?;

            if it.peek().is_some() {
                if let MappingKind::Inline { .. } = &self.kind {
                    write!(o, ",")?;
                }
            }
        }

        if let MappingKind::Inline { trailing, suffix } = &self.kind {
            if *trailing {
                write!(o, ",")?;
            }

            o.write_all(data.str(*suffix))?;
            write!(o, "}}")?;
        }

        Ok(())
    }
}

/// An element in a YAML mapping.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct MappingItem {
    pub(crate) key: String,
    pub(crate) value: Id,
}

impl MappingItem {
    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        self.key.display(data, f)?;
        write!(f, ":")?;
        data.raw(self.value).display(data, f, Some(self.value))?;
        Ok(())
    }

    #[cfg(feature = "std")]
    fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        self.key.write_to(data, o)?;
        write!(o, ":")?;
        o.write_all(data.prefix(self.value))?;
        data.raw(self.value).write_to(data, o)?;
        Ok(())
    }
}
