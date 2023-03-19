use core::fmt;

use crate::yaml::data::{Data, ValueId};
use crate::yaml::sequence::Iter;
use crate::yaml::Value;

/// Accessor for a sequence.
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
/// let root = doc.root().as_sequence().ok_or("missing root sequence")?;
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
/// let root = doc.root().as_sequence().ok_or("missing root sequence")?;
///
/// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
/// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
///
/// let three = root
///     .get(2)
///     .and_then(|v| v.as_sequence())
///     .ok_or("missing three")?;
///
/// assert_eq!(three.get(0).and_then(|v| v.as_str()), Some("three"));
///
/// let four = three
///     .get(1)
///     .and_then(|v| v.as_mapping())
///     .ok_or("missing four")?;
///
/// assert_eq!(four.get("four").and_then(|v| v.as_u32()), Some(2));
/// assert_eq!(four.get("five").and_then(|v| v.as_u32()), Some(1));
///
/// assert_eq!(root.get(3).and_then(|v| v.as_str()), Some("six"));
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
///
/// Sequences can also be defined in an inline form:
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::from_bytes("[]")?;
/// assert_eq!(doc.to_string(), "[]");
///
/// let doc = yaml::from_bytes("[,]")?;
/// let sequence = doc.root().as_sequence().ok_or("missing root sequence")?;
/// assert!(!sequence.is_empty());
/// assert_eq!(sequence.len(), 1);
/// assert_eq!(doc.to_string(), "[,]");
///
/// let doc = yaml::from_bytes(
///     r#"
///     [one, two, 3,]
///     "#,
/// )?;
///
/// let root = doc.root().as_sequence().ok_or("missing root sequence")?;
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
pub struct Sequence<'a> {
    data: &'a Data,
    pub(crate) id: ValueId,
}

impl<'a> Sequence<'a> {
    pub(crate) fn new(data: &'a Data, id: ValueId) -> Self {
        Self { data, id }
    }

    /// Get the opaque [`ValueId`] associated with this sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(r#"
    /// - 32
    /// - [1, 2, 3]
    /// "#)?;
    ///
    /// let root = doc.root().as_sequence().ok_or("missing sequence")?;
    /// let second = root.get(1).and_then(|v| v.as_sequence()).ok_or("missing second")?;
    /// let id = second.id();
    ///
    /// // Reference the same value again using the id.
    /// let second = doc.value(id).as_sequence().ok_or("missing id")?;
    /// assert!(second.iter().flat_map(|v| v.as_u32()).eq([1, 2, 3]));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn id(&self) -> ValueId {
        self.id
    }

    /// Get the length of the sequence.
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
    /// let root = doc.root().as_sequence().ok_or("missing root sequence")?;
    /// assert_eq!(root.len(), 3);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.data.sequence(self.id).items.len()
    }

    /// Test if the sequence is empty.
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
    /// let root = doc.root().as_sequence().ok_or("missing root sequence")?;
    /// assert!(!root.is_empty());
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.sequence(self.id).items.is_empty()
    }

    /// Get a value from the sequence.
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
    /// let root = doc.root().as_sequence().ok_or("missing root sequence")?;
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn get(&self, index: usize) -> Option<Value<'_>> {
        let item = self.data.sequence(self.id).items.get(index)?;
        let item = self.data.sequence_item(*item);
        Some(Value::new(self.data, item.value))
    }

    /// Returns an iterator over the sequence.
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
    /// let root = doc.root().as_sequence().ok_or("missing root sequence")?;
    /// root.iter().flat_map(|v| v.as_str()).eq(["one", "two", "three"]);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self.data, &self.data.sequence(self.id).items)
    }
}

impl fmt::Display for Sequence<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.sequence(self.id).display(self.data, f)
    }
}

impl fmt::Debug for Sequence<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// Returns an iterator over the [Sequence].
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
/// let root = doc.root().as_sequence().ok_or("missing root sequence")?;
/// root.into_iter().flat_map(|v| v.as_str()).eq(["one", "two", "three"]);
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
impl<'a> IntoIterator for Sequence<'a> {
    type Item = Value<'a>;
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self.data, &self.data.sequence(self.id).items)
    }
}
