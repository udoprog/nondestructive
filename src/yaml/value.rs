use core::fmt;

use bstr::ByteSlice;

use crate::slab::Pointer;
use crate::yaml::raw::Raw;
use crate::yaml::Document;

use super::raw::RawTable;

/// The kind of string value.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum StringKind {
    /// A bare string without quotes, such as `hello-world`.
    Bare,
    /// A single-quoted string.
    SingleQuoted,
    /// A double-quoted string.
    DoubleQuoted,
}

impl StringKind {
    /// Detect the appropriate kind to use for the given string.
    pub(crate) fn detect(string: &str) -> StringKind {
        let mut kind = StringKind::Bare;

        for c in string.bytes() {
            match (kind, c) {
                (_, b'\'') => {
                    return StringKind::DoubleQuoted;
                }
                (StringKind::Bare, c) => {
                    if !matches!(c, id_remainder!()) {
                        kind = StringKind::SingleQuoted;
                    }
                }
                _ => {}
            }
        }

        kind
    }
}

/// The kind of a null value.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum NullKind {
    /// A keyword `null` value.
    Keyword,
    /// A tilde `~` null value.
    Tilde,
    /// A empty null value.
    Empty,
}

/// A value inside of the document.
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::parse("string")?;
/// assert_eq!(doc.root().as_str(), Some("string"));
///
/// let doc = yaml::parse("\"a double-quoted string\"")?;
/// assert_eq!(doc.root().as_str(), Some("a double-quoted string"));
///
/// let doc = yaml::parse("'a single-quoted string'")?;
/// assert_eq!(doc.root().as_str(), Some("a single-quoted string"));
///
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
pub struct Value<'a> {
    doc: &'a Document,
    pointer: Pointer,
}

macro_rules! as_number {
    ($name:ident, $ty:ty, $doc:literal, $lit:literal) => {
        #[doc = concat!("Try and get the value as a ", $doc, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        #[doc = concat!("let doc = yaml::parse(\"", stringify!($lit), "\")?;")]
        #[doc = concat!("let value = doc.root().", stringify!($name), "();")]
        #[doc = concat!("assert_eq!(value, Some(", stringify!($lit), "));")]
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&self) -> Option<$ty> {
            match self.raw() {
                Some(Raw::Number(raw)) => {
                    let string = self.doc.strings.get(&raw.string);
                    lexical_core::parse(string).ok()
                }
                _ => None,
            }
        }
    };
}

impl<'a> Value<'a> {
    pub(crate) fn new(doc: &'a Document, pointer: Pointer) -> Self {
        Self { doc, pointer }
    }

    /// Get the raw element based on the value pointer.
    pub(crate) fn raw(&self) -> Option<&Raw> {
        self.doc.tree.get(&self.pointer)
    }

    /// Get the value as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse("string")?;
    /// let value = doc.root().as_str();
    /// assert_eq!(value, Some("string"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn as_str(&self) -> Option<&'a str> {
        match self.raw() {
            Some(Raw::String(raw)) => self.doc.strings.get(&raw.string).to_str().ok(),
            _ => None,
        }
    }

    /// Get the value as a table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse(r#"
    /// number1: 10
    /// number2: 20
    /// table:
    ///   inner: 400
    /// string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let root = doc.root().as_table().ok_or("missing root table")?;
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    ///
    /// let table = root.get("table").and_then(|v| v.as_table()).ok_or("missing inner table")?;
    /// assert_eq!(table.get("inner").and_then(|v| v.as_u32()), Some(400));
    ///
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn as_table(&self) -> Option<Table<'a>> {
        match self.raw() {
            Some(Raw::Table(..)) => Some(Table::new(self.doc, self.pointer)),
            _ => None,
        }
    }

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

impl fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(raw) = self.raw() else {
            return Ok(());
        };

        raw.display(self.doc, f)
    }
}

impl fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Display<T>(T);

        impl<T> fmt::Debug for Display<T>
        where
            T: fmt::Display,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        f.debug_struct("Value")
            .field("value", &Display(self))
            .finish_non_exhaustive()
    }
}

/// Accessor for a table.
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::parse(r#"
/// number1: 10
/// number2: 20
/// table:
///   inner: 400
/// string3: "I am a quoted string!"
/// "#)?;
///
/// let root = doc.root().as_table().ok_or("missing root table")?;
///
/// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
/// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
///
/// let table = root.get("table").and_then(|v| v.as_table()).ok_or("missing inner table")?;
/// assert_eq!(table.get("inner").and_then(|v| v.as_u32()), Some(400));
///
/// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
pub struct Table<'a> {
    doc: &'a Document,
    pointer: Pointer,
}

impl<'a> Table<'a> {
    pub(crate) fn new(doc: &'a Document, pointer: Pointer) -> Self {
        Self { doc, pointer }
    }

    /// Get the raw element based on the value pointer.
    pub(crate) fn raw(&self) -> Option<&RawTable> {
        match self.doc.tree.get(&self.pointer) {
            Some(Raw::Table(table)) => Some(table),
            _ => None,
        }
    }

    /// Get a value from the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse(r#"
    /// number1: 10
    /// number2: 20
    /// table:
    ///   inner: 400
    /// string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let root = doc.root().as_table().ok_or("missing root table")?;
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    ///
    /// let table = root.get("table").and_then(|v| v.as_table()).ok_or("missing inner table")?;
    /// assert_eq!(table.get("inner").and_then(|v| v.as_u32()), Some(400));
    ///
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn get(&self, key: &str) -> Option<Value<'_>> {
        let raw = self.raw()?;

        for e in &raw.children {
            if self.doc.strings.get(&e.key.string) == key {
                return Some(Value::new(self.doc, e.value));
            }
        }

        None
    }
}
