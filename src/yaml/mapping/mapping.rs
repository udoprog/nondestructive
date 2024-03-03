use core::fmt;

use bstr::BStr;

use crate::yaml::data::{Data, Id};
use crate::yaml::mapping::Iter;
use crate::yaml::Value;

/// Accessor for a mapping.
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice(
///     r#"
///     number1: 10
///     number2: 20
///     mapping:
///         inner: 400
///     string3: "I am a quoted string!"
///     "#
/// )?;
///
/// let root = doc.as_ref().as_mapping().context("missing root mapping")?;
///
/// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
/// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
///
/// let mapping = root.get("mapping").and_then(|v| v.as_mapping()).context("missing inner mapping")?;
/// assert_eq!(mapping.get("inner").and_then(|v| v.as_u32()), Some(400));
///
/// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
/// # Ok::<_, anyhow::Error>(())
/// ```
///
/// Mappings can also be defined in an inline form:
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice("{}")?;
/// assert_eq!(doc.to_string(), "{}");
///
/// let doc = yaml::from_slice("{test: 1,}")?;
/// let mapping = doc.as_ref().as_mapping().context("missing root mapping")?;
/// assert!(!mapping.is_empty());
/// assert_eq!(mapping.len(), 1);
/// assert_eq!(doc.to_string(), "{test: 1,}");
///
/// let doc = yaml::from_slice(
///     r#"
///     {one: one, two: two, three: 3,}
///     "#,
/// )?;
///
/// let root = doc.as_ref().as_mapping().context("missing root mapping")?;
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
/// # Ok::<_, anyhow::Error>(())
/// ```
pub struct Mapping<'a> {
    data: &'a Data,
    pub(crate) id: Id,
}

impl<'a> Mapping<'a> {
    pub(crate) fn new(data: &'a Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Get the opaque [`Id`] associated with this mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing mapping")?;
    /// let second = root.get("second").and_then(|v| v.as_sequence()).context("missing second")?;
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

    /// Get the length of the mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     three: 3
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    /// assert_eq!(root.len(), 3);
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.data.mapping(self.id).items.len()
    }

    /// Test if the mapping is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     three: 3
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    /// assert!(!root.is_empty());
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.mapping(self.id).items.is_empty()
    }

    /// Get a value from the mapping by its key.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     number1: 10
    ///     number2: 20
    ///     mapping:
    ///         inner: 400
    ///     string3: "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    ///
    /// let mapping = root.get("mapping").and_then(|v| v.as_mapping()).context("missing inner mapping")?;
    /// assert_eq!(mapping.get("inner").and_then(|v| v.as_u32()), Some(400));
    ///
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn get(&self, key: &str) -> Option<Value<'a>> {
        for item in &self.data.mapping(self.id).items {
            let item = self.data.mapping_item(*item);

            if self.data.str(item.key.string) == key {
                return Some(Value::new(self.data, item.value));
            }
        }

        None
    }

    /// Returns an iterator over the [Mapping].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     three: 3
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    /// root.iter().flat_map(|(key, value)| value.as_u32()).eq([1, 2, 3]);
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn iter(&self) -> Iter<'a> {
        Iter::new(self.data, &self.data.mapping(self.id).items)
    }
}

impl fmt::Display for Mapping<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.mapping(self.id).display(self.data, f)
    }
}

impl fmt::Debug for Mapping<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Returns an iterator over the [Mapping].
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice(
///     r#"
///     one: 1
///     two: 2
///     three: 3
///     "#,
/// )?;
///
/// let root = doc.as_ref().as_mapping().context("missing root mapping")?;
/// root.into_iter().flat_map(|(key, value)| value.as_u32()).eq([1, 2, 3]);
/// # Ok::<_, anyhow::Error>(())
/// ```
impl<'a> IntoIterator for Mapping<'a> {
    type Item = (&'a BStr, Value<'a>);
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Returns an iterator over the [Mapping].
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice(
///     r#"
///     one: 1
///     two: 2
///     three: 3
///     "#,
/// )?;
///
/// let root = doc.as_ref().as_mapping().context("missing root mapping")?;
/// (&root).into_iter().flat_map(|(key, value)| value.as_u32()).eq([1, 2, 3]);
/// # Ok::<_, anyhow::Error>(())
/// ```
impl<'a> IntoIterator for &Mapping<'a> {
    type Item = (&'a BStr, Value<'a>);
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
