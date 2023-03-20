//! Errors associated with processing YAML.

use core::fmt;
use core::ops::Range;

/// An error raised by the YAML module.
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
    /// Bad a sequence terminator.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// const INPUT: &str = r#"[Aristotle, # this is a comment"#;
    ///
    /// let error = yaml::from_slice(INPUT).unwrap_err();
    /// assert_eq!(*error.kind(), yaml::ErrorKind::BadSequenceTerminator);
    /// assert_eq!(&INPUT[error.span()], " # this is a comment");
    /// ```
    BadSequenceTerminator,
    /// Bad mapping separator.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// const INPUT: &str = r#"{name: Aristotle, age # this is a comment"#;
    ///
    /// let error = yaml::from_slice(INPUT).unwrap_err();
    /// assert_eq!(*error.kind(), yaml::ErrorKind::BadMappingSeparator);
    /// assert_eq!(&INPUT[error.span()], " age # this is a comment");
    /// ```
    ///
    /// Missing terminator in a non-inline mapping:
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// const INPUT: &str = r#"
    /// name: Aristotle
    /// age # end"#;
    ///
    /// let error = yaml::from_slice(INPUT).unwrap_err();
    /// assert_eq!(*error.kind(), yaml::ErrorKind::BadMappingSeparator);
    /// assert_eq!(&INPUT[error.span()], "age # end");
    /// ```
    BadMappingSeparator,
    /// Bad a mapping terminator.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// const INPUT: &str = r#"{name: Aristotle, # this is a comment"#;
    ///
    /// let error = yaml::from_slice(INPUT).unwrap_err();
    /// assert_eq!(*error.kind(), yaml::ErrorKind::BadMappingTerminator);
    /// assert_eq!(&INPUT[error.span()], " # this is a comment");
    /// ```
    BadMappingTerminator,
    /// Not a valid escape sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// const INPUT: &str = r#""hello \o1u world""#;
    ///
    /// let error = yaml::from_slice(INPUT).unwrap_err();
    /// assert_eq!(*error.kind(), yaml::ErrorKind::BadEscape);
    /// assert_eq!(&INPUT[error.span()], "\\o");
    /// ```
    BadEscape,
    /// Bad hex escape.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// const INPUT: &str = r#""hello \x1u world""#;
    ///
    /// let error = yaml::from_slice(INPUT).unwrap_err();
    /// assert_eq!(*error.kind(), yaml::ErrorKind::BadHexEscape);
    /// assert_eq!(&INPUT[error.span()], "\\x1u");
    /// ```
    BadHexEscape,
    /// Bad unicode escape.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// const INPUT: &str = r#""hello \ud800 world""#;
    ///
    /// let error = yaml::from_slice(INPUT).unwrap_err();
    /// assert_eq!(*error.kind(), yaml::ErrorKind::BadUnicodeEscape);
    /// assert_eq!(&INPUT[error.span()], "\\ud800");
    /// ```
    BadUnicodeEscape,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::BadSequenceTerminator => write!(f, "bad sequence terminator"),
            ErrorKind::BadMappingSeparator => write!(f, "bad mapping separator"),
            ErrorKind::BadMappingTerminator => write!(f, "bad mapping terminator"),
            ErrorKind::BadEscape => write!(f, "bad escape"),
            ErrorKind::BadHexEscape => write!(f, "bad hex escape"),
            ErrorKind::BadUnicodeEscape => write!(f, "bad unicode escape"),
        }
    }
}
