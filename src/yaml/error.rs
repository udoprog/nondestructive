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
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let error = yaml::from_bytes("私").unwrap_err();
    /// assert_eq!(error.span(), 0..1);
    /// ```
    #[must_use]
    #[inline]
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::error::Error for Error {}

/// The kind of a parse error.
#[derive(Debug)]
pub(crate) enum ErrorKind {
    /// Failed to parse value.
    ValueError,
    /// Expected table separator.
    ExpectedTableSeparator,
    /// Expected list,
    ExpectedListMarker,
    /// Expected a list terminator.
    ExpectedListTerminator,
    /// Expected a table terminator.
    ExpectedTableTerminator,
    /// Expected valid escape sequence.
    ExpectedEscape,
    /// Expected hex escape.
    ExpectedHexEscape,
    /// Expected unicode escape.
    ExpectedUnicodeEscape,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::ValueError => write!(f, "value error"),
            ErrorKind::ExpectedTableSeparator => write!(f, "expected table separator"),
            ErrorKind::ExpectedListMarker => write!(f, "expected list marker"),
            ErrorKind::ExpectedListTerminator => write!(f, "expected list terminator"),
            ErrorKind::ExpectedTableTerminator => write!(f, "expected table terminator"),
            ErrorKind::ExpectedEscape => write!(f, "expected escape"),
            ErrorKind::ExpectedHexEscape => write!(f, "expected hex escape"),
            ErrorKind::ExpectedUnicodeEscape => write!(f, "expected unicode escape"),
        }
    }
}
