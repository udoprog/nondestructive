use std::fmt;

use serde::de;

/// A error raised during deserialization.
///
/// See [`serde` module][crate::yaml::serde] for documentation.
#[derive(Debug)]
pub struct Error {
    inner: de::value::Error,
}

impl de::Error for Error {
    #[inline]
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self {
            inner: de::value::Error::custom(msg.to_string()),
        }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for Error {}
