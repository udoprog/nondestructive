use std::fmt;
use std::io;

use bstr::{BStr, ByteSlice};

use crate::yaml::data::Data;
use crate::yaml::raw::Raw;
use crate::yaml::{Any, Mapping, Sequence};

use super::data::Id;

/// The kind of a multiline string.
#[derive(Default, Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Chomp {
    /// This is the `-` chomping indicator, which strips the final line break.
    Strip,
    /// This is the default chomping indicator, which keeps a single final line
    /// break.
    #[default]
    Clip,
    /// This is the `+` chomping indicator, which keeps any final line breaks as
    /// they are considered apart of the content.
    Keep,
}

impl Chomp {
    pub(crate) fn as_byte(self) -> Option<u8> {
        match self {
            Chomp::Strip => Some(b'-'),
            Chomp::Clip => None,
            Chomp::Keep => Some(b'+'),
        }
    }
}

/// The kind of a string.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum StringKind {
    /// A bare string, without any escaping.
    Bare,
    /// A single-quoted string.
    ///
    /// ```yaml
    /// 'Hello World'
    /// ```
    Single,
    /// A double-quoted string.
    ///
    /// ```yaml
    /// "Hello\nWorld"
    /// ```
    Double,
}

/// The kind of a block.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Block {
    /// The literal `|` block, with a chomping mode as indicated by [`Chomp`].
    ///
    /// ```yaml
    /// |
    ///   Hello
    ///   World
    /// ```
    Literal(Chomp),
    /// The folded `>` block, with a chomping mode as indicated by [`Chomp`].
    /// Line breaks are converted into a single `' '` space.
    ///
    /// ```yaml
    /// >
    ///   Hello
    ///   World
    /// ```
    Folded(Chomp),
}

/// Separator to use when separating the value from its key or sequence marker.
///
/// ```yaml
/// -   hello
/// - world
/// ```
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Separator<'a> {
    /// Automatically figure out which separator to use based on the last
    /// element in the collection.
    ///
    /// If this does not exist, a default separator of `" "` will be used.
    Auto,
    /// A custom separator.
    ///
    /// # Legal separators
    ///
    /// The only legal separator in YAML is spaces, but this can technically
    /// contain anything and will be literally embedded in the generated YAML.
    /// It is up to the caller to ensure nothing but spaces is used or suffer
    /// the consequences.
    Custom(&'a str),
}

/// The kind of a null value.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Null {
    /// A keyword `null` value.
    Keyword,
    /// A tilde `~` null value.
    Tilde,
    /// A empty null value.
    Empty,
}

impl Null {
    pub(crate) fn display(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Null::Keyword => {
                write!(f, "null")?;
            }
            Null::Tilde => {
                write!(f, "~")?;
            }
            Null::Empty => {
                // empty values count as null.
            }
        }

        Ok(())
    }

    pub(crate) fn write_to<O>(self, o: &mut O) -> io::Result<()>
    where
        O: ?Sized + io::Write,
    {
        match self {
            Null::Keyword => {
                write!(o, "null")?;
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

/// A value inside of the document.
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice("string")?;
/// assert_eq!(doc.as_ref().as_str(), Some("string"));
///
/// let doc = yaml::from_slice("\"a double-quoted string\"")?;
/// assert_eq!(doc.as_ref().as_str(), Some("a double-quoted string"));
///
/// let doc = yaml::from_slice("'a single-quoted string'")?;
/// assert_eq!(doc.as_ref().as_str(), Some("a single-quoted string"));
///
/// let doc = yaml::from_slice("'It''s a bargain!'")?;
/// assert_eq!(doc.as_ref().as_str(), Some("It's a bargain!"));
///
/// # Ok::<_, anyhow::Error>(())
/// ```
pub struct Value<'a> {
    pub(crate) data: &'a Data,
    pub(crate) id: Id,
}

macro_rules! as_number {
    ($name:ident, $ty:ty, $doc:literal, $lit:literal) => {
        #[doc = concat!("Try and get the value as a ", $doc, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use anyhow::Context;
        /// use nondestructive::yaml;
        ///
        #[doc = concat!("let doc = yaml::from_slice(\"", stringify!($lit), "\")?;")]
        #[doc = concat!("let value = doc.as_ref().", stringify!($name), "();")]
        #[doc = concat!("assert_eq!(value, Some(", stringify!($lit), "));")]
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        #[must_use]
        pub fn $name(&self) -> Option<$ty> {
            match self.data.raw(self.id) {
                Raw::Number(raw) => {
                    let string = self.data.str(raw.string);
                    lexical_core::parse(string).ok()
                }
                _ => None,
            }
        }
    };
}

impl<'a> Value<'a> {
    pub(crate) fn new(data: &'a Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Coerce into [`Any`] to help discriminate the value type.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     Hello World
    ///     "#
    /// )?;
    ///
    /// assert!(matches!(doc.as_ref().into_any(), yaml::Any::Scalar(..)));
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     number1: 10
    ///     number2: 20
    ///     "#
    /// )?;
    ///
    /// assert!(matches!(doc.as_ref().into_any(), yaml::Any::Mapping(..)));
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - 10
    ///     - 20
    ///     "#
    /// )?;
    ///
    /// assert!(matches!(doc.as_ref().into_any(), yaml::Any::Sequence(..)));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn into_any(self) -> Any<'a> {
        match self.data.raw(self.id) {
            Raw::Mapping(..) => Any::Mapping(Mapping::new(self.data, self.id)),
            Raw::Sequence(..) => Any::Sequence(Sequence::new(self.data, self.id)),
            _ => Any::Scalar(self),
        }
    }

    /// Get the opaque [`Id`] associated with this value.
    ///
    /// This can be used through [`Document::value`] to look up the same value
    /// again.
    ///
    /// [`Document::value`]: crate::yaml::Document::value
    ///
    /// # Panics
    ///
    /// Values constructed from identifiers might cause panics if used
    /// incorrectly, such as when it refers to a value which has been deleted.
    ///
    /// ```should_panic
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing mapping")?;
    /// let second = root.get("second").context("missing second")?;
    /// let id = second.id();
    ///
    /// let mut root = doc.as_mut().into_mapping_mut().context("missing mapping")?;
    /// assert!(root.remove("second"));
    ///
    /// // This will panic:
    /// let _ = doc.value(id).as_mapping();
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing mapping")?;
    /// let second = root.get("second").context("missing second")?;
    /// let id = second.id();
    ///
    /// // Reference the same value again using the id.
    /// assert_eq!(doc.value(id).to_string(), "[1, 2, 3]");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn id(&self) -> Id {
        self.id
    }

    /// Get the value as a [`BStr`].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    /// use bstr::BStr;
    ///
    /// let doc = yaml::from_slice("string")?;
    /// assert_eq!(doc.as_ref().as_str(), Some("string"));
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - It's the same string!
    ///     - "It's the same string!"
    ///     - 'It''s the same string!'
    ///     "#
    /// )?;
    ///
    /// let array = doc.as_ref().as_sequence().context("expected sequence")?;
    ///
    /// for item in array {
    ///     assert_eq!(item.as_bstr(), Some(BStr::new("It's the same string!")));
    /// }
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn as_bstr(&self) -> Option<&'a BStr> {
        match self.data.raw(self.id) {
            Raw::String(raw) => Some(self.data.str(raw.string)),
            _ => None,
        }
    }

    /// Get the value as a [`str`]. This might fail if the underlying string is
    /// not valid UTF-8.
    ///
    /// See [`Value::as_bstr`] for an alternative.
    ///
    /// # Escape sequences and unicode
    ///
    /// YAML supports a variety of escape sequences which will be handled by
    /// this parser.
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("フェリスと言います！")?;
    /// assert_eq!(doc.as_ref().as_str(), Some("フェリスと言います！"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("\"hello \\x20 world\"")?;
    /// assert_eq!(doc.as_ref().as_str(), Some("hello \x20 world"));
    /// assert_eq!(doc.to_string(), "\"hello \\x20 world\"");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("string")?;
    /// assert_eq!(doc.as_ref().as_str(), Some("string"));
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - It's the same string!
    ///     - "It's the same string!"
    ///     - 'It''s the same string!'
    ///     "#
    /// )?;
    ///
    /// let array = doc.as_ref().as_sequence().context("expected sequence")?;
    ///
    /// for item in array {
    ///     assert_eq!(item.as_str(), Some("It's the same string!"));
    /// }
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> Option<&'a str> {
        match self.data.raw(self.id) {
            Raw::String(raw) => self.data.str(raw.string).to_str().ok(),
            _ => None,
        }
    }

    /// Get the value as a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("true")?;
    /// assert_eq!(doc.as_ref().as_bool(), Some(true));
    ///
    /// let doc = yaml::from_slice("string")?;
    /// assert_eq!(doc.as_ref().as_bool(), None);
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self.data.raw(self.id) {
            Raw::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// Get the value as a [`Mapping`].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     number1: 10
    ///     number2: 20
    ///     mapping:
    ///         inner: 400
    ///     string3: "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    ///
    /// let mapping = root.get("mapping").and_then(|v| v.as_mapping()).context("missing inner mapping")?;
    /// assert_eq!(mapping.get("inner").and_then(|v| v.as_u32()), Some(400));
    ///
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn as_mapping(&self) -> Option<Mapping<'a>> {
        match self.data.raw(self.id) {
            Raw::Mapping(..) => Some(Mapping::new(self.data, self.id)),
            _ => None,
        }
    }

    /// Get the value as a [`Sequence`].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn as_sequence(&self) -> Option<Sequence<'a>> {
        match self.data.raw(self.id) {
            Raw::Sequence(..) => Some(Sequence::new(self.data, self.id)),
            _ => None,
        }
    }

    as_number!(as_f32, f32, "32-bit float", 10.42);
    as_number!(as_f64, f64, "64-bit float", 10.42);
    as_number!(as_u8, u8, "8-bit unsigned integer", 42);
    as_number!(as_i8, i8, "8-bit signed integer", -42);
    as_number!(as_u16, u16, "16-bit unsigned integer", 42);
    as_number!(as_i16, i16, "16-bit signed integer", -42);
    as_number!(as_u32, u32, "16-bit unsigned integer", 42);
    as_number!(as_i32, i32, "32-bit signed integer", -42);
    as_number!(as_u64, u64, "16-bit unsigned integer", 42);
    as_number!(as_i64, i64, "64-bit signed integer", -42);
    as_number!(as_u128, u128, "16-bit unsigned integer", 42);
    as_number!(as_i128, i128, "128-bit signed integer", -42);
}

impl fmt::Display for Value<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.raw(self.id).display(self.data, f)
    }
}

impl fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Display<'a, 'b>(&'a Value<'b>);

        impl fmt::Debug for Display<'_, '_> {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.data.raw(self.0.id).display(self.0.data, f)
            }
        }

        f.debug_tuple("Value").field(&Display(self)).finish()
    }
}
