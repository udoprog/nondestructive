use core::fmt;

use crate::yaml::data::{Data, ValueId};
use crate::yaml::list::Iter;
use crate::yaml::Value;

/// Accessor for a list.
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::from_bytes(
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
/// let doc = yaml::from_bytes(
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
///
/// Lists can also be defined in an inline form:
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::from_bytes("[]")?;
/// assert_eq!(doc.to_string(), "[]");
///
/// let doc = yaml::from_bytes("[,]")?;
/// let list = doc.root().as_list().ok_or("missing root list")?;
/// assert!(!list.is_empty());
/// assert_eq!(list.len(), 1);
/// assert_eq!(doc.to_string(), "[,]");
///
/// let doc = yaml::from_bytes(
///     r#"
///     [one, two, 3,]
///     "#,
/// )?;
///
/// let root = doc.root().as_list().ok_or("missing root list")?;
/// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
/// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
/// assert_eq!(root.get(2).and_then(|v| v.as_u32()), Some(3));
///
/// assert_eq!(
///     doc.to_string(),
///     r#"
///     [one, two, 3,]
///     "#
/// );
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
pub struct List<'a> {
    data: &'a Data,
    id: ValueId,
}

impl<'a> List<'a> {
    pub(crate) fn new(data: &'a Data, id: ValueId) -> Self {
        Self { data, id }
    }

    /// Get the length of the list.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(
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
        self.data.list(self.id).items.len()
    }

    /// Test if the list is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(
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
        self.data.list(self.id).items.is_empty()
    }

    /// Get a value from the list.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(
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
        let item = self.data.list(self.id).items.get(index)?;
        Some(Value::new(self.data, item.value))
    }

    /// Returns an iterator over the list.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_list().ok_or("missing root list")?;
    /// root.iter().flat_map(|v| v.as_str()).eq(["one", "two", "three"]);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self.data, &self.data.list(self.id).items)
    }
}

impl fmt::Display for List<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.list(self.id).display(self.data, f)
    }
}

impl fmt::Debug for List<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// Returns an iterator over the [List].
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::from_bytes(
///     r#"
///     - one
///     - two
///     - three
///     "#,
/// )?;
///
/// let root = doc.root().as_list().ok_or("missing root list")?;
/// root.into_iter().flat_map(|v| v.as_str()).eq(["one", "two", "three"]);
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
impl<'a> IntoIterator for List<'a> {
    type Item = Value<'a>;
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self.data, &self.data.list(self.id).items)
    }
}
