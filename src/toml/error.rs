//! Errors associated with processing TOML.

use core::fmt;
use core::ops::Range;

/// An error raised by the TOML module.
#[derive(Debug)]
pub struct Error {
    span: Range<usize>,
    kind: ErrorKind,
}

impl Error {
    /// Construct a new error.
    #[inline]
    pub(crate) const fn new(span: Range<usize>, kind: ErrorKind) -> Self {
        Self { span, kind }
    }

    /// Get the range of the input span.
    #[must_use]
    #[inline]
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// Get the kind of an error.
    #[must_use]
    #[inline]
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (at {}-{})",
            self.kind, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for Error {}

/// The kind of an [`Error`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// Expect end of file.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::toml;
    ///
    /// const INPUT: &str = r#"
    /// {hello: world}
    /// 42
    /// "#;
    ///
    /// let error = toml::from_slice(INPUT).unwrap_err();
    /// assert_eq!(*error.kind(), toml::ErrorKind::ExpectedEof);
    /// assert_eq!(&INPUT[error.span()], "42\n");
    /// ```
    ExpectedEof,
    /// Expected a table separator.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::toml;
    ///
    /// const INPUT: &str = r#"
    /// foo    bar
    /// "#;
    ///
    /// let error = toml::from_slice(INPUT).unwrap_err();
    /// assert_eq!(*error.kind(), toml::ErrorKind::ExpectedSeparator);
    /// assert_eq!(&INPUT[error.span()], "    ");
    /// ```
    ExpectedSeparator,
    /// Expected a number.
    ExpectedNumber,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::ExpectedEof => write!(f, "expected end-of-file"),
            ErrorKind::ExpectedSeparator => write!(f, "expected table separator"),
            ErrorKind::ExpectedNumber => write!(f, "expected number"),
        }
    }
}
