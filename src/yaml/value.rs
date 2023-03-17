use core::fmt;

use bstr::ByteSlice;

use crate::strings::Strings;
use crate::yaml::raw::{Raw, RawKind, RawList, RawTable};

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

/// Separator to use when separating the value from its key or list marker.
///
/// ```yaml
/// -   hello
/// - world
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Separator<'a> {
    /// Automatically figure out which separator to use based on the last
    /// element in the collection.
    ///
    /// If this does not exist, a default separator of `" "` will be used.
    Auto,
    /// A custom separator.
    Custom(&'a str),
}

impl StringKind {
    /// Detect the appropriate kind to use for the given string.
    pub(crate) fn detect(string: &str) -> StringKind {
        let mut kind = StringKind::Bare;

        for c in string.chars() {
            match c {
                '\'' => {
                    return StringKind::DoubleQuoted;
                }
                ':' => {
                    kind = StringKind::SingleQuoted;
                }
                b if b.is_control() => {
                    return StringKind::DoubleQuoted;
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

impl NullKind {
    pub(crate) fn display(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NullKind::Keyword => {
                write!(f, "null")?;
            }
            NullKind::Tilde => {
                write!(f, "~")?;
            }
            NullKind::Empty => {
                // empty values count as null.
            }
        }

        Ok(())
    }
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
    strings: &'a Strings,
    raw: &'a Raw,
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
        #[must_use]
        #[inline]
        pub fn $name(&self) -> Option<$ty> {
            match &self.raw.kind {
                RawKind::Number(raw) => {
                    let string = self.strings.get(&raw.string);
                    lexical_core::parse(string).ok()
                }
                _ => None,
            }
        }
    };
}

impl<'a> Value<'a> {
    pub(crate) fn new(strings: &'a Strings, raw: &'a Raw) -> Self {
        Self { strings, raw }
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
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> Option<&'a str> {
        match &self.raw.kind {
            RawKind::String(raw) => self.strings.get(&raw.string).to_str().ok(),
            _ => None,
        }
    }

    /// Get the value as a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse("true")?;
    /// assert_eq!(doc.root().as_bool(), Some(true));
    ///
    /// let doc = yaml::parse("string")?;
    /// assert_eq!(doc.root().as_bool(), None);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        const TRUE: &[u8] = b"true";
        const FALSE: &[u8] = b"false";

        match &self.raw.kind {
            RawKind::String(raw) => match (raw.kind, self.strings.get(&raw.string).as_bytes()) {
                (StringKind::Bare, TRUE) => Some(true),
                (StringKind::Bare, FALSE) => Some(false),
                _ => None,
            },
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
    #[must_use]
    #[inline]
    pub fn as_table(&self) -> Option<Table<'a>> {
        match &self.raw.kind {
            RawKind::Table(raw) => Some(Table::new(self.strings, raw)),
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
    /// let doc = yaml::parse(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_list().ok_or("missing root list")?;
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn as_list(&self) -> Option<List<'a>> {
        match &self.raw.kind {
            RawKind::List(raw) => Some(List::new(self.strings, raw)),
            _ => None,
        }
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

impl fmt::Display for Value<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.display(self.strings, f)
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
    strings: &'a Strings,
    raw: &'a RawTable,
}

impl<'a> Table<'a> {
    pub(crate) fn new(strings: &'a Strings, raw: &'a RawTable) -> Self {
        Self { strings, raw }
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
    #[must_use]
    #[inline]
    pub fn get(&self, key: &str) -> Option<Value<'a>> {
        for e in &self.raw.items {
            if self.strings.get(&e.key.string) == key {
                return Some(Value::new(self.strings, &e.value));
            }
        }

        None
    }
}

impl fmt::Display for Table<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.display(self.strings, f)
    }
}

/// Accessor for a list.
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::parse(
///     r#"
///     - one
///     - two
///     - three
///     "#,
/// )?;
///
/// let root = doc.root().as_list().ok_or("missing root list")?;
///
/// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
/// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
/// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
///
/// More complex example:
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::parse(
///     r#"
///     - one
///     - two
///     - - three
///       - four: 2
///         five: 1
///     - six
///     "#,
/// )?;
///
/// let root = doc.root().as_list().ok_or("missing root list")?;
///
/// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
/// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
///
/// let three = root
///     .get(2)
///     .and_then(|v| v.as_list())
///     .ok_or("missing three")?;
///
/// assert_eq!(three.get(0).and_then(|v| v.as_str()), Some("three"));
///
/// let four = three
///     .get(1)
///     .and_then(|v| v.as_table())
///     .ok_or("missing four")?;
///
/// assert_eq!(four.get("four").and_then(|v| v.as_u32()), Some(2));
/// assert_eq!(four.get("five").and_then(|v| v.as_u32()), Some(1));
///
/// assert_eq!(root.get(3).and_then(|v| v.as_str()), Some("six"));
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
pub struct List<'a> {
    strings: &'a Strings,
    raw: &'a RawList,
}

impl<'a> List<'a> {
    pub(crate) fn new(strings: &'a Strings, raw: &'a RawList) -> Self {
        Self { strings, raw }
    }

    /// Get the length of the list.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_list().ok_or("missing root list")?;
    /// assert_eq!(root.len(), 3);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.raw.items.len()
    }

    /// Test if the list is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_list().ok_or("missing root list")?;
    /// assert!(!root.is_empty());
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.raw.items.is_empty()
    }

    /// Get a value from the list.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_list().ok_or("missing root list")?;
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn get(&self, index: usize) -> Option<Value<'_>> {
        let item = self.raw.items.get(index)?;
        Some(Value::new(self.strings, &item.value))
    }
}

impl fmt::Display for List<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.display(self.strings, f)
    }
}
