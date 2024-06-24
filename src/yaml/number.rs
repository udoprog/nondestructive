use core::fmt;

use bstr::{BStr, ByteSlice};

use crate::yaml::data::Data;
use crate::yaml::raw;

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
        /// let value = doc.as_ref().into_number().context("expected a number")?;
        #[doc = concat!("let value = value.", stringify!($name), "();")]
        #[doc = concat!("assert_eq!(value, Some(", stringify!($lit), "));")]
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        #[must_use]
        pub fn $name(&self) -> Option<$ty> {
            let string = self.data.str(self.raw.string);
            lexical_core::parse(string).ok()
        }
    };
}

/// A YAML number.
///
/// The value of the number can be accessed through the various `as_*` methods.
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use bstr::ByteSlice;
/// use nondestructive::yaml;
///
/// let a = yaml::from_slice("42")?;
/// let a = a.as_ref();
///
/// assert_eq!(a.as_u32(), Some(42));
/// # Ok::<_, anyhow::Error>(())
/// ```
pub struct Number<'a> {
    data: &'a Data,
    raw: &'a raw::Number,
}

impl<'a> Number<'a> {
    pub(super) fn new(data: &'a Data, raw: &'a raw::Number) -> Self {
        Self { data, raw }
    }

    /// Get the raw content of the number.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("3.1415")?;
    /// let doc = doc.as_ref();
    /// let value = doc.as_number().context("expected a number")?;
    /// let value = value.as_raw();
    /// assert_eq!(value, "3.1415");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn as_raw(&self) -> &BStr {
        self.data.str(self.raw.string)
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

impl fmt::Debug for Number<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(string) = self.as_raw().to_str() {
            f.write_str(string)
        } else {
            f.write_str("NaN")
        }
    }
}
