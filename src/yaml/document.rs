use std::fmt;
use std::io;

#[cfg(feature = "serde-edits")]
use serde::{Deserialize, Serialize};

use crate::yaml::data::{Data, Id, StringId};
use crate::yaml::{Value, ValueMut};

/// A whitespace preserving YAML document.
#[derive(Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
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
    ///     r"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "
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
    ///     r"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "
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
    ///     r"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "
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
    ///     r"
    ///     first: 32
    ///     second: [1, 2, 3]
    ///     "
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
    ///     r"
    ///     first: 32
    ///     second: Hello World
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn value_mut(&mut self, id: Id) -> ValueMut<'_> {
        ValueMut::new(&mut self.data, id)
    }

    /// Write the bytes of the document to the given `output`.
    ///
    /// # Errors
    ///
    /// Raises an I/O error if the underlying resource being written to raises
    /// it.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     string
    ///     "
    /// )?;
    ///
    /// let mut mapping = doc.as_mut().make_mapping();
    /// mapping.insert_u32("first", 1);
    /// mapping.insert_u32("second", 2);
    ///
    /// let mut out = Vec::new();
    /// doc.write_to(&mut out)?;
    ///
    /// assert_eq!(
    ///     &out[..],
    ///     br"
    ///     first: 1
    ///     second: 2
    ///     "
    /// );
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     first: second
    ///     "
    /// )?;
    ///
    /// let mut mapping = doc.as_mut().into_mapping_mut().and_then(|m| Some(m.get_into_mut("first")?.make_mapping())).context("missing first")?;
    /// mapping.insert_u32("second", 2);
    /// mapping.insert_u32("third", 3);
    ///
    /// let mut out = Vec::new();
    /// doc.write_to(&mut out)?;
    ///
    /// assert_eq!(
    ///     &out[..],
    ///     br"
    ///     first:
    ///       second: 2
    ///       third: 3
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn write_to<O>(&self, mut output: O) -> io::Result<()>
    where
        O: io::Write,
    {
        output.write_all(self.data.prefix(self.root))?;
        self.data.raw(self.root).write_to(&self.data, &mut output)?;
        output.write_all(self.data.str(self.suffix))?;
        Ok(())
    }

    // Display helper for document.
    fn display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::fmt::Display;

        self.data.prefix(self.root).fmt(f)?;
        self.data.raw(self.root).display(&self.data, f, None)?;
        self.data.str(self.suffix).fmt(f)?;
        Ok(())
    }
}

impl fmt::Display for Document {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // If we're running in debug mode, assert that the produced document
        // matches whatever would've been produced through `Document::write_to`.
        //
        // This is only enabled with `--cfg nondestructive_write_to_eq`.
        if cfg!(nondestructive_write_to_eq) {
            use bstr::BStr;
            use std::fmt::Write;

            #[repr(transparent)]
            struct Inner<'a>(&'a Document);

            impl fmt::Display for Inner<'_> {
                #[inline]
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.0.display(f)
                }
            }

            let mut string = String::new();
            write!(string, "{}", Inner(self))?;
            let mut bytes = Vec::new();

            self.write_to(&mut bytes)
                .expect("Document::write_to should not panic");

            debug_assert_eq!(
                BStr::new(string.as_bytes()),
                BStr::new(&bytes),
                "nondestructive_write_to_eq: ensure write_to produces the same output"
            );

            string.fmt(f)?;
        } else {
            self.display(f)?;
        }

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
