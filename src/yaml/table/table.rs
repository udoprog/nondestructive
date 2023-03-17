use core::fmt;

use bstr::BStr;

use crate::strings::Strings;
use crate::yaml::raw::RawTable;
use crate::yaml::table::Iter;
use crate::yaml::Value;

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
///
/// Tables can also be defined in an inline form:
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::parse("{}")?;
/// assert_eq!(doc.to_string(), "{}");
///
/// let doc = yaml::parse("{test: 1,}")?;
/// let table = doc.root().as_table().ok_or("missing root table")?;
/// assert!(!table.is_empty());
/// assert_eq!(table.len(), 1);
/// assert_eq!(doc.to_string(), "{test: 1,}");
///
/// let doc = yaml::parse(
///     r#"
///     {one: one, two: two, three: 3,}
///     "#,
/// )?;
///
/// let root = doc.root().as_table().ok_or("missing root table")?;
/// assert_eq!(root.get("one").and_then(|v| v.as_str()), Some("one"));
/// assert_eq!(root.get("two").and_then(|v| v.as_str()), Some("two"));
/// assert_eq!(root.get("three").and_then(|v| v.as_u32()), Some(3));
///
/// assert_eq!(
///     doc.to_string(),
///     r#"
///     {one: one, two: two, three: 3,}
///     "#
/// );
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

    /// Get the length of the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse(
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     three: 3
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_table().ok_or("missing root table")?;
    /// assert_eq!(root.len(), 3);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.raw.items.len()
    }

    /// Test if the table is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse(
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     three: 3
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_table().ok_or("missing root table")?;
    /// assert!(!root.is_empty());
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.raw.items.is_empty()
    }

    /// Get a value from the table by its key.
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

    /// Returns an iterator over the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse(
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     three: 3
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_table().ok_or("missing root table")?;
    /// root.iter().flat_map(|(key, value)| value.as_u32()).eq([1, 2, 3]);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self.strings, &self.raw.items)
    }

    /// Returns an iterator over the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse(
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     three: 3
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_table().ok_or("missing root table")?;
    /// root.into_iter().flat_map(|(key, value)| value.as_u32()).eq([1, 2, 3]);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn into_iter(self) -> Iter<'a> {
        Iter::new(self.strings, &self.raw.items)
    }
}

impl fmt::Display for Table<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.display(self.strings, f)
    }
}

impl fmt::Debug for Table<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'a> IntoIterator for Table<'a> {
    type Item = (&'a BStr, Value<'a>);
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Table::into_iter(self)
    }
}
