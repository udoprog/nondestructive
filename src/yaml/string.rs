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

    /// Get the raw contents of the string, as it's being referenced during
    /// parsing or insertion.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let a = yaml::from_slice(r#""Hello\n World""#)?;
    /// let a = a.as_ref().into_any().into_string().context("expected string")?;
    /// let a = a.as_raw();
    /// assert_eq!(a, "\"Hello\\n World\"");
    ///
    /// let b = yaml::from_slice(r#""Hello World""#)?;
    /// let b = b.as_ref().into_any().into_string().context("expected string")?;
    /// let b = b.as_raw();
    /// assert_eq!(b, "\"Hello World\"");
    ///
    /// let c = yaml::from_slice("'Hello World'")?;
    /// let c = c.as_ref().into_any().into_string().context("expected string")?;
    /// let c = c.as_raw();
    /// assert_eq!(c, "'Hello World'");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn as_raw(&self) -> &BStr {
        self.data.str(self.raw.original)
    }
}

impl Deref for String<'_> {
    type Target = BStr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data.str(self.raw.id)
    }
}

impl fmt::Debug for String<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.str(self.raw.id).fmt(f)
    }
}
