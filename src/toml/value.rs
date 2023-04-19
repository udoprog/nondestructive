use bstr::ByteSlice;

use crate::toml::data::{Data, Id, StringId};
use crate::toml::raw::Raw;

/// A value inside of a [`Document`][crate::toml::Document].
///
/// # Examples
///
/// ```
/// use nondestructive::toml;
///
/// let doc = toml::from_slice("key = string")?;
/// assert_eq!(doc.as_ref().get("key").and_then(|v| v.as_str()), Some("string"));
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
        /// use nondestructive::toml;
        ///
        #[doc = concat!("let doc = toml::from_slice(\"key = ", stringify!($lit), "\")?;")]
        #[doc = concat!("let value = doc.as_ref().get(\"key\").and_then(|v| v.", stringify!($name), "());")]
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
    #[inline]
    pub(crate) fn new(data: &'a Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Get the value as a [`str`]. This might fail if the underlying string is
    /// not valid UTF-8.
    ///
    /// See [`Value::as_bstr`] for an alternative.
    ///
    /// # Escape sequences and unicode
    ///
    /// TOML supports a variety of escape sequences which will be handled by
    /// this parser.
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::toml;
    ///
    /// let doc = toml::from_slice("key = \"フェリスと言います！\"")?;
    /// assert_eq!(doc.as_ref().get("key").and_then(|v| v.as_str()), Some("フェリスと言います！"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::toml;
    ///
    /// let doc = toml::from_slice(
    ///     r#"
    ///     key = [
    ///         "It's the same string!"
    ///         'It''s the same string!'
    ///     ]
    ///     "#
    /// )?;
    ///
    /// let array = doc.as_ref().get("key").and_then(|v| v.as_sequence()).context("expected sequence")?;
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
