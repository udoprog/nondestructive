use crate::yaml::{Mapping, Number, Sequence, String};

/// An enum which helps to externally discriminate the interior type of a
/// [`Value`].
///
/// See [`Value::into_any`] or [`Value::as_any`].
///
/// [`Value::into_any`]: crate::yaml::Value::into_any
/// [`Value::as_any`]: crate::yaml::Value::as_any
#[derive(Debug)]
#[non_exhaustive]
pub enum Any<'a> {
    /// A null value.
    Null,
    /// An boolean value.
    Bool(bool),
    /// A number value.
    Number(Number<'a>),
    /// A string value.
    String(String<'a>),
    /// A [`Mapping`] value.
    Mapping(Mapping<'a>),
    /// A [`Sequence`] value.
    Sequence(Sequence<'a>),
}

impl<'a> Any<'a> {
    /// Test if [`Any`] is null.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use bstr::ByteSlice;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("null")?;
    /// let doc = doc.as_ref();
    ///
    /// assert!(doc.as_any().is_null());
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn is_null(self) -> bool {
        matches!(self, Self::Null)
    }

    /// Coerce into [`Any`] into a bool.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use bstr::ByteSlice;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("true")?;
    /// let doc = doc.as_ref();
    ///
    /// let value = doc.as_any().into_bool().context("expected bool")?;
    /// assert_eq!(value, true);
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn into_bool(self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Coerce into [`Any`] into a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use bstr::ByteSlice;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(r#""Hello World""#)?;
    /// let doc = doc.as_ref();
    ///
    /// let value = doc.as_any().into_string().context("expected string")?;
    /// assert_eq!(value.to_str(), Ok("Hello World"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn into_string(self) -> Option<String<'a>> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// Coerce into [`Any`] into a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use bstr::ByteSlice;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("42")?;
    /// let doc = doc.as_ref();
    ///
    /// let value = doc.as_any().into_number().context("expected number")?;
    /// assert_eq!(value.as_u32(), Some(42));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn into_number(self) -> Option<Number<'a>> {
        match self {
            Self::Number(value) => Some(value),
            _ => None,
        }
    }
}
