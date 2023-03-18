use core::fmt;

use bstr::BStr;

use crate::yaml::data::{Data, ValueId};
use crate::yaml::table::Iter;
use crate::yaml::Value;

/// Accessor for a table.
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::from_bytes(r#"
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
/// let doc = yaml::from_bytes("{}")?;
/// assert_eq!(doc.to_string(), "{}");
///
/// let doc = yaml::from_bytes("{test: 1,}")?;
/// let table = doc.root().as_table().ok_or("missing root table")?;
/// assert!(!table.is_empty());
/// assert_eq!(table.len(), 1);
/// assert_eq!(doc.to_string(), "{test: 1,}");
///
/// let doc = yaml::from_bytes(
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
    data: &'a Data,
    id: ValueId,
}

impl<'a> Table<'a> {
    pub(crate) fn new(data: &'a Data, id: ValueId) -> Self {
        Self { data, id }
    }

    /// Get the opaque [`ValueId`] associated with this table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(r#"
    /// first: 32
    /// second: [1, 2, 3]
    /// "#)?;
    ///
    /// let root = doc.root().as_table().ok_or("missing table")?;
    /// let second = root.get("second").and_then(|v| v.as_list()).ok_or("missing second")?;
    /// let id = second.id();
    ///
    /// // Reference the same value again using the id.
    /// let second = doc.value(id).as_list().ok_or("missing id")?;
    /// assert!(second.iter().flat_map(|v| v.as_u32()).eq([1, 2, 3]));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn id(&self) -> ValueId {
        self.id
    }

    /// Get the length of the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(
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
        self.data.table(self.id).items.len()
    }

    /// Test if the table is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(
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
        self.data.table(self.id).items.is_empty()
    }

    /// Get a value from the table by its key.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(r#"
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
        for item in &self.data.table(self.id).items {
            if self.data.str(&item.key.string) == key {
                return Some(Value::new(self.data, item.value));
            }
        }

        None
    }

    /// Returns an iterator over the [Table].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(
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
    #[must_use]
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self.data, &self.data.table(self.id).items)
    }
}

impl fmt::Display for Table<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.table(self.id).display(self.data, f)
    }
}

impl fmt::Debug for Table<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Returns an iterator over the [Table].
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::from_bytes(
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
impl<'a> IntoIterator for Table<'a> {
    type Item = (&'a BStr, Value<'a>);
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self.data, &self.data.table(self.id).items)
    }
}
