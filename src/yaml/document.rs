use core::fmt;

use crate::yaml::data::{Data, Id, StringId};
use crate::yaml::{Value, ValueMut};

/// A whitespace preserving YAML document.
#[derive(Clone)]
pub struct Document {
    suffix: StringId,
    pub(crate) root: Id,
    pub(crate) data: Data,
}

impl Document {
    /// Construct a new document.
    pub(crate) fn new(suffix: StringId, root: Id, data: Data) -> Self {
        Self { suffix, root, data }
    }

    /// Get the document as a [`Value`].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("32")?;
    /// assert_eq!(doc.as_ref().as_u32(), Some(32));
    ///
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn as_ref(&self) -> Value<'_> {
        Value::new(&self.data, self.root)
    }

    /// Get the document as a [`ValueMut`].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice("  32")?;
    /// doc.as_mut().set_u32(42);
    /// assert_eq!(doc.to_string(), "  42");
    ///
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn as_mut(&mut self) -> ValueMut<'_> {
        ValueMut::new(&mut self.data, self.root)
    }

    /// Get the given value.
    ///
    /// If [`Id`]'s are shared between documents, this might also result in
    /// unspecified behavior, such as it referencing a random value in the other
    /// document.
    ///
    /// # Panics
    ///
    /// Values constructed from identifiers might cause panics if used
    /// incorrectly, such as when it refers to a value which has been deleted.
    ///
    /// ```should_panic
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing mapping")?;
    /// let second = root.get("second").context("missing second")?;
    /// let id = second.id();
    ///
    /// let mut root = doc.as_mut().into_mapping_mut().context("missing mapping")?;
    /// assert!(root.remove("second"));
    ///
    /// // This will panic:
    /// let _ = doc.value(id).as_mapping();
    /// # Ok::<_, anyhow::Error>(())
    /// ```
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
    /// let second = root.get("second").context("missing second")?;
    /// let id = second.id();
    ///
    /// // Reference the same value again using the id.
    /// assert_eq!(doc.value(id).to_string(), "[1, 2, 3]");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn value(&self, id: Id) -> Value<'_> {
        Value::new(&self.data, id)
    }

    /// Get the given value mutably.
    ///
    /// If [`Id`]'s are shared between documents, this might also result in
    /// unspecified behavior, such as it referencing a random value in the other
    /// document.
    ///
    /// # Panics
    ///
    /// Values constructed from identifiers might cause panics if used
    /// incorrectly, such as when it refers to a value which has been deleted.
    ///
    /// ```should_panic
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing mapping")?;
    /// let second = root.get("second").context("missing second")?;
    /// let id = second.id();
    ///
    /// let mut root = doc.as_mut().into_mapping_mut().context("missing mapping")?;
    /// assert!(root.remove("second"));
    ///
    /// // This will panic:
    /// let _ = doc.value_mut(id).into_mapping_mut();
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref().as_mapping().context("missing mapping")?;
    /// let second = root.get("second").context("missing second")?;
    /// let id = second.id();
    ///
    /// // Reference the same value again using the id.
    /// doc.value_mut(id).set_string("Hello World");
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     first: 32
    ///     second: Hello World
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn value_mut(&mut self, id: Id) -> ValueMut<'_> {
        ValueMut::new(&mut self.data, id)
    }
}

impl fmt::Display for Document {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.prefix(self.root).fmt(f)?;
        self.data.raw(self.root).display(&self.data, f)?;
        self.data.str(self.suffix).fmt(f)?;
        Ok(())
    }
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("suffix", &self.suffix)
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}
