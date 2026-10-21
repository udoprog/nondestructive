use core::fmt::{self, Write};

use alloc::vec::Vec;

use std::io;

use bstr::ByteSlice;
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

/// Construct a block to be inserted as an item of the mapping or sequence `id`.
pub(crate) fn new_block<I>(data: &mut Data, id: Id, iter: I, block: Block) -> Raw
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let parent = container_indent(data, id);
    let indent = parent.map_or(2, |p| p.wrapping_add(2));
    build_block(data, &[], indent, parent, iter, block)
}

/// Construct a block to replace the value `id`.
pub(crate) fn new_value_block<I>(data: &mut Data, id: Id, iter: I, block: Block) -> Raw
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let parent = data
        .layout(id)
        .parent
        .and_then(|item| data.layout(item).parent)
        .and_then(|container| container_indent(data, container));

    let prefix = data.str(data.layout(id).prefix);

    // A value on a line of its own is indented relative to that line, otherwise
    // relative to the container it belongs to.
    let (line, indent) = match prefix.rfind([NEWLINE]) {
        Some(n) => {
            let line = prefix.get(n.wrapping_add(1)..).unwrap_or_default().to_vec();
            let indent = line.len().wrapping_add(2);
            (line, indent)
        }
        None => (Vec::new(), parent.map_or(2, |p| p.wrapping_add(2))),
    };

    build_block(data, &line, indent, parent, iter, block)
}

/// Fit the block built at `id` by [`new_block`] or [`new_value_block`] to the
/// text that follows it now that it is placed.
///
/// A block reads its final line break from the following text, so one is
/// added to the document suffix when the block ends the document without one.
/// A keeping block also takes the empty lines that follow it into its value.
pub(crate) fn settle_block(data: &mut Data, id: Id) {
    let Raw::String(string) = data.raw(id) else {
        return;
    };

    let original = string.original;
    let mut value = data.str(string.id).to_vec();

    let keep = data
        .str(original)
        .iter()
        .take_while(|&&b| b != NEWLINE)
        .any(|&b| b == b'+');

    let (mut following, end) = following(data, id);

    if end {
        if following.is_empty()
            && value.last() == Some(&NEWLINE)
            && data.suffix().first() != Some(&NEWLINE)
        {
            let mut suffix = Vec::new();
            suffix.push(NEWLINE);
            suffix.extend_from_slice(data.suffix());
            let suffix = data.insert_str(suffix);
            data.set_suffix(suffix);
        }

        following.extend_from_slice(data.suffix());
    }

    if !keep {
        return;
    }

    let Some(rest) = following.strip_prefix(&[NEWLINE]) else {
        return;
    };

    // Each following line of only spaces is an empty line of the block.
    let empty = rest
        .split_inclusive(|&b| b == NEWLINE)
        .take_while(|line| {
            line.split_last()
                .is_some_and(|(&last, line)| last == NEWLINE && line.iter().all(|&b| b == SPACE))
        })
        .count();

    if empty > 0 {
        value.resize(value.len().wrapping_add(empty), NEWLINE);
        let value = data.insert_str(value);
        data.replace(
            id,
            Raw::String(self::String::new(RawStringKind::Original, value, original)),
        );
    }
}

/// The text between the value `id` and the next item of an enclosing
/// collection, and whether the document suffix follows that text instead.
fn following(data: &Data, mut id: Id) -> (Vec<u8>, bool) {
    let mut text = Vec::new();

    while let Some(item) = data.layout(id).parent {
        let separator = match data.raw(item) {
            Raw::MappingItem(item) => item.separator_prefix,
            Raw::SequenceItem(item) => item.separator_prefix,
            _ => return (text, false),
        };

        text.extend_from_slice(data.str(separator));

        let Some(container) = data.layout(item).parent else {
            return (text, false);
        };

        let items = match data.raw(container) {
            Raw::Mapping(raw) => &raw.items,
            Raw::Sequence(raw) => &raw.items,
            _ => return (text, false),
        };

        let next = items
            .iter()
            .position(|&i| i == item)
            .and_then(|n| items.get(n.wrapping_add(1)));

        if let Some(&next) = next {
            text.extend_from_slice(data.prefix(next));
            return (text, false);
        }

        id = container;
    }

    (text, true)
}

/// The indentation of a block mapping or sequence.
fn container_indent(data: &Data, id: Id) -> Option<usize> {
    match data.raw(id) {
        Raw::Mapping(raw) => Some(raw.indent),
        Raw::Sequence(raw) => Some(raw.indent),
        _ => None,
    }
}

/// Build a block whose content lines start with `line` followed by spaces up
/// to `indent` columns, nested in a collection indented by `parent`.
///
/// The value is what parsing the block gives when a line break follows it.
fn build_block<I>(
    data: &mut Data,
    line: &[u8],
    mut indent: usize,
    parent: Option<usize>,
    iter: I,
    block: Block,
) -> Raw
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut parts = Vec::new();

    for part in iter {
        parts.extend(part.as_ref().split('\n').map(|s| s.as_bytes().to_vec()));
    }

    let (mark, folded, chomp) = match block {
        Block::Literal(chomp) => (b'|', false, chomp),
        Block::Folded(chomp) => (b'>', true, chomp),
    };

    let is_spaced = |part: &[u8]| matches!(part.first(), Some(b' ' | b'\t'));

    // Leading whitespace on the first content line would be taken as
    // indentation, so state the indentation explicitly instead.
    let explicit = parts
        .iter()
        .find(|part| !part.is_empty())
        .is_some_and(|part| is_spaced(part));

    // The indentation indicator counts from the parent collection.
    let anchor = parent.unwrap_or(0);
    let mut line = line;

    if explicit && !(1..=9).contains(&indent.wrapping_sub(anchor)) {
        line = &[];
        indent = anchor.wrapping_add(2);
    }

    let digit = explicit
        .then(|| u8::try_from(indent.wrapping_sub(anchor)).ok())
        .flatten()
        .map(|d| b'0'.wrapping_add(d));

    let mut original = Vec::new();
    original.push(mark);
    original.extend(digit);
    original.extend(chomp.as_byte());

    let mut out = Vec::new();
    // Line breaks pending since the last content line.
    let mut breaks = 0usize;
    // Whether the last content line was more indented, if there was one.
    let mut spaced = None;

    for part in &parts {
        original.push(NEWLINE);

        if part.is_empty() {
            breaks = breaks.wrapping_add(1);
            continue;
        }

        original.extend_from_slice(line);
        original.resize(
            original
                .len()
                .wrapping_add(indent.saturating_sub(line.len())),
            SPACE,
        );
        original.extend_from_slice(part);

        let is_spaced = is_spaced(part);

        if folded && spaced == Some(false) && !is_spaced {
            if breaks == 1 {
                out.push(SPACE);
            } else {
                out.resize(out.len().wrapping_add(breaks.saturating_sub(1)), NEWLINE);
            }
        } else {
            out.resize(out.len().wrapping_add(breaks), NEWLINE);
        }

        out.extend_from_slice(part);
        spaced = Some(is_spaced);
        breaks = 1;
    }

    let trailing = match chomp {
        Chomp::Clip if spaced.is_some() => breaks.min(1),
        Chomp::Strip | Chomp::Clip => 0,
        Chomp::Keep => breaks,
    };

    out.resize(out.len().wrapping_add(trailing), NEWLINE);

    let original = data.insert_str(&original);
    let string = data.insert_str(out);

    Raw::String(self::String::new(RawStringKind::Original, string, original))
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
pub(crate) struct Layout {
    /// Reference to the indentation just preceding the current value.
    pub(crate) prefix: StringId,
    /// Reference to the parent of a value.
    pub(crate) parent: Option<Id>,
}

/// A raw value.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(tag = "kind", content = "value"))]
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
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

fn write_prefix<O>(data: &Data, o: &mut O, prefix: Option<Id>) -> io::Result<()>
where
    O: ?Sized + io::Write,
{
    if let Some(id) = prefix {
        o.write_all(data.prefix(id))?;
    }

    Ok(())
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

    pub(crate) fn write_to<O>(&self, data: &Data, o: &mut O, prefix: Option<Id>) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        match self {
            Raw::Null(raw) => {
                raw.write_to(data, o, prefix)?;
            }
            Raw::Mapping(raw) => {
                raw.write_to(data, o, prefix)?;
            }
            Raw::Sequence(raw) => {
                raw.write_to(data, o, prefix)?;
            }
            Raw::Boolean(raw) => {
                write_prefix(data, o, prefix)?;
                raw.write_to(data, o)?;
            }
            Raw::Number(raw) => {
                write_prefix(data, o, prefix)?;
                raw.write_to(data, o)?;
            }
            Raw::String(raw) => {
                write_prefix(data, o, prefix)?;
                raw.write_to(data, o)?;
            }
            Raw::MappingItem(raw) => {
                write_prefix(data, o, prefix)?;
                raw.write_to(data, o)?;
            }
            Raw::SequenceItem(raw) => {
                write_prefix(data, o, prefix)?;
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
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
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

    pub(crate) fn write_to<O>(&self, data: &Data, o: &mut O, prefix: Option<Id>) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        match self {
            Null::Keyword(string) => {
                write_prefix(data, o, prefix)?;
                o.write_all(data.str(*string))?;
            }
            Null::Tilde => {
                write_prefix(data, o, prefix)?;
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
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
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
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
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

/// The kind of string value.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(tag = "kind"))]
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
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
    ///
    /// A string is written [`Bare`] when [`is_plain_safe`] holds, and quoted
    /// otherwise. [`Double`] is used whenever the string needs escaping or
    /// contains a `'`, [`Single`] for anything else.
    ///
    /// [`Bare`]: RawStringKind::Bare
    /// [`Single`]: RawStringKind::Single
    /// [`Double`]: RawStringKind::Double
    pub(crate) fn detect(string: &str) -> RawStringKind {
        if string.chars().any(|c| needs_escape(c) || c == '\'') {
            RawStringKind::Double
        } else if is_plain_safe(string) {
            RawStringKind::Bare
        } else {
            RawStringKind::Single
        }
    }
}

/// Detect the kind to write a mapping key `key` as.
///
/// Keys follow the same rules as values through [`RawStringKind::detect`],
/// which already quotes strings starting with `?` or containing `": "`. Keys
/// that are not valid UTF-8 are written bare.
pub(crate) fn key_kind(key: &[u8]) -> RawStringKind {
    match core::str::from_utf8(key) {
        Ok(key) => RawStringKind::detect(key),
        Err(..) => RawStringKind::Bare,
    }
}

/// The plain spellings of null under the YAML 1.2 core schema.
pub(crate) const NULL: [&[u8]; 4] = [b"~", b"null", b"Null", b"NULL"];
/// The plain spellings of `true` under the YAML 1.2 core schema.
pub(crate) const TRUE: [&[u8]; 3] = [b"true", b"True", b"TRUE"];
/// The plain spellings of `false` under the YAML 1.2 core schema.
pub(crate) const FALSE: [&[u8]; 3] = [b"false", b"False", b"FALSE"];

/// Whether a character can only be written escaped in a double-quoted string.
fn needs_escape(c: char) -> bool {
    c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{feff}')
}

/// Whether `string` can be written as a plain scalar in any block or flow
/// context and read back as the same string, both by this crate and by
/// libyaml.
///
/// This excludes strings that would resolve to null, a boolean or a number,
/// are empty or have surrounding whitespace, start with an indicator or a
/// document marker, contain a flow indicator, `": "` or `" #"`, end with `:`,
/// or contain a character that needs escaping.
pub(crate) fn is_plain_safe(string: &str) -> bool {
    let bytes = string.as_bytes();

    let (Some(&first), Some(&last)) = (bytes.first(), bytes.last()) else {
        return false;
    };

    if matches!(first, b' ' | b'?' | b':' | b'#' | b'&' | b'*' | b'!' | b'|')
        || matches!(first, b'>' | b'\'' | b'"' | b'%' | b'@' | b'`')
        || matches!(last, b' ' | b':')
        || matches!(bytes, [b'-', b' ', ..] | [b'-'])
        || bytes.starts_with(b"---")
        || bytes.starts_with(b"...")
    {
        return false;
    }

    if bytes
        .iter()
        .any(|b| matches!(b, b',' | b'[' | b']' | b'{' | b'}'))
        || string.contains(": ")
        || string.contains(" #")
        || string.chars().any(needs_escape)
    {
        return false;
    }

    !resolves_as_non_string(bytes)
}

/// Whether a plain scalar would resolve to something other than a string.
///
/// Anything that starts like a number (an optional sign followed by a digit,
/// or by `.` and a digit) is treated as one, which covers the forms libyaml
/// reads as numbers beyond the core schema, such as `0b101` and `+0x1f`.
fn resolves_as_non_string(bytes: &[u8]) -> bool {
    if NULL.contains(&bytes) || TRUE.contains(&bytes) || FALSE.contains(&bytes) {
        return true;
    }

    if crate::yaml::number::resolve(bytes).is_some() {
        return true;
    }

    let unsigned = match bytes {
        [b'+' | b'-', rest @ ..] => rest,
        _ => bytes,
    };

    matches!(unsigned, [b'0'..=b'9', ..] | [b'.', b'0'..=b'9', ..])
}

/// A YAML string.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
pub(crate) struct String {
    /// The kind of the string.
    pub(crate) kind: RawStringKind,
    /// The content of the string.
    pub(crate) id: StringId,
    /// The original contents of the string.
    pub(crate) original: StringId,
}

impl String {
    /// A YAML string.
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
                    '\\' => "\\\\",
                    '\"' => "\\\"",
                    '\u{85}' => "\\N",
                    '\u{A0}' => "\\_",
                    '\u{2028}' => "\\L",
                    '\u{2029}' => "\\P",
                    '\u{feff}' => "\\uFEFF",
                    c if c.is_control() => {
                        write!(f, "{}\\x{:02x}", &string[start..index], u32::from(c))?;
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

            for (index, end, c) in string.char_indices() {
                let esc: &[u8] = match c {
                    '\u{00}' => b"\\0",
                    '\u{07}' => b"\\a",
                    '\u{08}' => b"\\b",
                    '\u{09}' => b"\\t",
                    '\n' => b"\\n",
                    '\u{0b}' => b"\\v",
                    '\u{0c}' => b"\\f",
                    '\r' => b"\\r",
                    '\u{1b}' => b"\\e",
                    '\\' => b"\\\\",
                    '\"' => b"\\\"",
                    '\u{85}' => b"\\N",
                    '\u{A0}' => b"\\_",
                    '\u{2028}' => b"\\L",
                    '\u{2029}' => b"\\P",
                    '\u{feff}' => b"\\uFEFF",
                    c if c.is_control() => {
                        o.write_all(&string[s..index])?;
                        write!(o, "\\x{:02x}", u32::from(c))?;
                        s = end;
                        continue;
                    }
                    _ => {
                        continue;
                    }
                };

                o.write_all(&string[s..index])?;
                o.write_all(esc)?;
                s = end;
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
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
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
    /// [one, two, three]
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
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
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

    fn write_to<O>(&self, data: &Data, o: &mut O, prefix: Option<Id>) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        if matches!(self.kind, SequenceKind::Inline { .. }) || !self.items.is_empty() {
            write_prefix(data, o, prefix)?;
        }

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
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
pub(crate) struct SequenceItem {
    pub(crate) value: Id,
    /// Whitespace and comments between the value and a following flow `,`.
    pub(crate) separator_prefix: StringId,
}

impl SequenceItem {
    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        data.raw(self.value).display(data, f, Some(self.value))?;
        write!(f, "{}", data.str(self.separator_prefix))?;
        Ok(())
    }

    fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        data.raw(self.value).write_to(data, o, Some(self.value))?;
        o.write_all(data.str(self.separator_prefix))?;
        Ok(())
    }
}

/// The kind of a raw mapping.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(tag = "kind"))]
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
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
        /// The inner suffix, before the trailing `}`.
        suffix: StringId,
    },
}

/// A YAML mapping.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
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

    fn write_to<O>(&self, data: &Data, o: &mut O, prefix: Option<Id>) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        if matches!(self.kind, MappingKind::Inline { .. }) || !self.items.is_empty() {
            write_prefix(data, o, prefix)?;
        }

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
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
pub(crate) struct MappingItem {
    pub(crate) key: String,
    pub(crate) value: Id,
    /// Whitespace and comments between the value and a following flow `,`.
    pub(crate) separator_prefix: StringId,
}

impl MappingItem {
    fn display(&self, data: &Data, f: &mut fmt::Formatter) -> fmt::Result {
        self.key.display(data, f)?;
        write!(f, ":")?;
        data.raw(self.value).display(data, f, Some(self.value))?;
        write!(f, "{}", data.str(self.separator_prefix))?;
        Ok(())
    }

    fn write_to<O>(&self, data: &Data, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        self.key.write_to(data, o)?;
        write!(o, ":")?;
        data.raw(self.value).write_to(data, o, Some(self.value))?;
        o.write_all(data.str(self.separator_prefix))?;
        Ok(())
    }
}
