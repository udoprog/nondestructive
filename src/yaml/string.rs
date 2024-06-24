use core::fmt;
use core::ops::Deref;

use bstr::BStr;

use crate::yaml::data::Data;
use crate::yaml::raw;

/// A YAML string.
///
/// The string is accessed through a [`BStr`] coercion, since strings might
/// contain non-utf8 data.
///
/// Use utilities such as [`bstr::ByteSlice::to_str`] to coerce it into a
/// [`str`].
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use bstr::ByteSlice;
/// use nondestructive::yaml;
///
/// let a = yaml::from_slice(r#""Hello\n World""#)?;
/// let a = a.as_ref();
///
/// let a = a.as_any().into_string().context("expected string")?;
/// let a = a.to_str()?;
/// assert_eq!(a, "Hello\n World");
/// # Ok::<_, anyhow::Error>(())
/// ```
pub struct String<'a> {
    data: &'a Data,
    raw: &'a raw::String,
}

impl<'a> String<'a> {
    pub(super) fn new(data: &'a Data, raw: &'a raw::String) -> Self {
        Self { data, raw }
    }

    /// Get the raw contents of the string, as it's being referenced.
    ///
    /// For strings which contains an escape sequence, the whole string content
    /// will be referenced including any parenthesis.
    ///
    /// For other strings, only the contents of the string is referenced.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let a = yaml::from_slice(r#""Hello\n World""#)?;
    /// let a = a.as_ref();
    ///
    /// let a = a.as_any().into_string().context("expected string")?;
    /// let a = a.as_raw();
    /// assert_eq!(a, "\"Hello\\n World\"");
    ///
    /// let b = yaml::from_slice(r#""Hello World""#)?;
    /// let b = b.as_ref();
    ///
    /// let b = b.as_any().into_string().context("expected string")?;
    /// let b = b.as_raw();
    /// assert_eq!(b, "Hello World");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[inline]
    pub fn as_raw(&self) -> &BStr {
        match self.raw.kind {
            raw::RawStringKind::Original { original }
            | raw::RawStringKind::Multiline { original, .. } => self.data.str(original),
            _ => self.data.str(self.raw.string),
        }
    }
}

impl Deref for String<'_> {
    type Target = BStr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data.str(self.raw.string)
    }
}

impl fmt::Debug for String<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.str(self.raw.string).fmt(f)
    }
}
