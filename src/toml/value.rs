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
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("key = \"フェリスと言います！\"")?;
    /// assert_eq!(doc.as_ref().get("key").and_then(|v| v.as_str()), Some("フェリスと言います！"));
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
}
