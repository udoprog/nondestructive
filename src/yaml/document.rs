use core::fmt;

use crate::yaml::data::{Data, StringId, ValueId};
use crate::yaml::{Value, ValueMut};

/// A whitespace preserving YAML document.
#[derive(Clone)]
pub struct Document {
    prefix: StringId,
    suffix: StringId,
    pub(crate) root: ValueId,
    pub(crate) data: Data,
}

impl Document {
    /// Construct a new document.
    pub(crate) fn new(prefix: StringId, suffix: StringId, root: ValueId, data: Data) -> Self {
        Self {
            prefix,
            suffix,
            root,
            data,
        }
    }

    /// Get the root value of a document.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes("32")?;
    /// assert_eq!(doc.root().as_u32(), Some(32));
    ///
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn root(&self) -> Value<'_> {
        Value::new(&self.data, self.root)
    }

    /// Get the root value of a document.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes("  32")?;
    /// doc.root_mut().set_u32(42);
    /// assert_eq!(doc.to_string(), "  42");
    ///
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn root_mut(&mut self) -> ValueMut<'_> {
        ValueMut::new(&mut self.data, self.root)
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.str(&self.prefix).fmt(f)?;
        self.data.raw(self.root).display(&self.data, f)?;
        self.data.str(&self.suffix).fmt(f)?;
        Ok(())
    }
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("prefix", &self.prefix)
            .field("suffix", &self.suffix)
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}
