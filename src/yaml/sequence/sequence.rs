use core::fmt;

use crate::yaml::data::{Data, Id};
use crate::yaml::sequence::Iter;
use crate::yaml::Value;

/// Accessor for a sequence.
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice(
///     r#"
///     - one
///     - two
///     - three
///     "#,
/// )?;
///
/// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
///
/// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
/// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
/// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
/// # Ok::<_, anyhow::Error>(())
/// ```
///
/// More complex example:
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice(
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
/// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
///
/// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
/// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
///
/// let three = root
///     .get(2)
///     .and_then(|v| v.as_sequence())
///     .context("missing three")?;
///
/// assert_eq!(three.get(0).and_then(|v| v.as_str()), Some("three"));
///
/// let four = three
///     .get(1)
///     .and_then(|v| v.as_mapping())
///     .context("missing four")?;
///
/// assert_eq!(four.get("four").and_then(|v| v.as_u32()), Some(2));
/// assert_eq!(four.get("five").and_then(|v| v.as_u32()), Some(1));
///
/// assert_eq!(root.get(3).and_then(|v| v.as_str()), Some("six"));
/// # Ok::<_, anyhow::Error>(())
/// ```
///
/// Sequences can also be defined in an inline form:
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice("[]")?;
/// assert_eq!(doc.to_string(), "[]");
///
/// let doc = yaml::from_slice("[,]")?;
/// let sequence = doc.as_ref().as_sequence().context("missing root sequence")?;
/// assert!(!sequence.is_empty());
/// assert_eq!(sequence.len(), 1);
/// assert_eq!(doc.to_string(), "[,]");
///
/// let doc = yaml::from_slice(
///     r#"
///     [one, two, 3,]
///     "#,
/// )?;
///
/// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
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
/// # Ok::<_, anyhow::Error>(())
/// ```
pub struct Sequence<'a> {
    data: &'a Data,
    pub(crate) id: Id,
}

impl<'a> Sequence<'a> {
    pub(crate) fn new(data: &'a Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Get the opaque [`Id`] associated with this sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - 32
    ///     - [1, 2, 3]
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref().as_sequence().context("missing sequence")?;
    /// let second = root.get(1).and_then(|v| v.as_sequence()).context("missing second")?;
    /// let id = second.id();
    ///
    /// // Reference the same value again using the id.
    /// let second = doc.value(id).as_sequence().context("missing id")?;
    /// assert!(second.iter().flat_map(|v| v.as_u32()).eq([1, 2, 3]));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn id(&self) -> Id {
        self.id
    }

    /// Get the length of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
    /// assert_eq!(root.len(), 3);
    /// # Ok::<_, anyhow::Error>(())
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
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
    /// assert!(!root.is_empty());
    /// # Ok::<_, anyhow::Error>(())
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
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn get(&self, index: usize) -> Option<Value<'_>> {
        let item = self.data.sequence(self.id).items.get(index)?;
        let item = self.data.sequence_item(*item);
        Some(Value::new(self.data, item.value))
    }

    /// Get the first value of a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
    ///
    /// assert_eq!(root.first().and_then(|v| v.as_str()), Some("one"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn first(&self) -> Option<Value<'_>> {
        let item = self.data.sequence(self.id).items.first()?;
        let item = self.data.sequence_item(*item);
        Some(Value::new(self.data, item.value))
    }

    /// Get the last value of a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
    ///
    /// assert_eq!(root.last().and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn last(&self) -> Option<Value<'_>> {
        let item = self.data.sequence(self.id).items.last()?;
        let item = self.data.sequence_item(*item);
        Some(Value::new(self.data, item.value))
    }

    /// Returns an iterator over the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
    /// root.iter().flat_map(|v| v.as_str()).eq(["one", "two", "three"]);
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn iter(&self) -> Iter<'a> {
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
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice(
///     r#"
///     - one
///     - two
///     - three
///     "#,
/// )?;
///
/// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
/// root.into_iter().flat_map(|v| v.as_str()).eq(["one", "two", "three"]);
/// # Ok::<_, anyhow::Error>(())
/// ```
impl<'a> IntoIterator for Sequence<'a> {
    type Item = Value<'a>;
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Returns an iterator over the [Sequence].
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice(
///     r#"
///     - one
///     - two
///     - three
///     "#,
/// )?;
///
/// let root = doc.as_ref().as_sequence().context("missing root sequence")?;
/// (&root).into_iter().flat_map(|v| v.as_str()).eq(["one", "two", "three"]);
/// # Ok::<_, anyhow::Error>(())
/// ```
impl<'a> IntoIterator for &Sequence<'a> {
    type Item = Value<'a>;
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
