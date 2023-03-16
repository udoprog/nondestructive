use core::fmt;

use crate::slab::{Pointer, Slab};
use crate::strings::{StringId, Strings};
use crate::yaml::raw::Raw;
use crate::yaml::{Value, ValueMut};

/// A whitespace preserving YAML document.
#[derive(Clone)]
pub struct Document {
    prefix: StringId,
    suffix: StringId,
    root: Pointer,
    pub(crate) strings: Strings,
    pub(crate) tree: Slab<Raw>,
}

impl Document {
    /// Construct a new document.
    pub(crate) fn new(
        prefix: StringId,
        suffix: StringId,
        root: Pointer,
        strings: Strings,
        tree: Slab<Raw>,
    ) -> Self {
        Self {
            prefix,
            suffix,
            root,
            strings,
            tree,
        }
    }

    /// Get the root value of a document.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::parse("32")?;
    /// assert_eq!(doc.root().as_u32(), Some(32));
    ///
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn root(&self) -> Value<'_> {
        Value::new(self, self.root)
    }

    /// Get the root value of a document.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse("  32")?;
    /// doc.root_mut().set_u32(42);
    /// assert_eq!(doc.to_string(), "  42");
    ///
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn root_mut(&mut self) -> ValueMut<'_> {
        ValueMut::new(self, self.root)
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.strings.get(&self.prefix).fmt(f)?;

        if let Some(raw) = self.tree.get(&self.root) {
            raw.display(self, f)?;
        }

        self.strings.get(&self.suffix).fmt(f)?;
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
