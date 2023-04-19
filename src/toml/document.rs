use std::fmt;
use std::io;

#[cfg(feature = "serde-edits")]
use serde::{Deserialize, Serialize};

use crate::toml::data::{Data, Id, StringId};
use crate::toml::{Table, Value, ValueMut};

/// A whitespace preserving TOML document.
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
    /// use nondestructive::toml;
    ///
    /// let doc = toml::from_slice("key = 32")?;
    /// assert_eq!(doc.as_ref().as_u32(), Some(32));
    ///
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn as_ref(&self) -> Table<'_> {
        Table::new(&self.data, self.root)
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
    /// use nondestructive::toml;
    ///
    /// let mut doc = toml::from_slice(
    ///     r#"
    ///     first = "string"
    ///     "#
    /// )?;
    ///
    /// let mut mapping = doc.as_mut();
    /// mapping.insert_u32("first", 1);
    /// mapping.insert_u32("second", 2);
    ///
    /// let mut out = Vec::new();
    /// doc.write_to(&mut out)?;
    ///
    /// assert_eq!(
    ///     &out[..],
    ///     br#"
    ///     first = 1
    ///     second = 2
    ///     "#
    /// );
    ///
    /// let mut doc = toml::from_slice(
    ///     r#"
    ///     first = "second"
    ///     "#
    /// )?;
    ///
    /// let mut mapping = doc.as_mut().and_then(|m| Some(m.get_into_mut("first")?.make_table())).context("missing first")?;
    /// mapping.insert_u32("second", 2);
    /// mapping.insert_u32("third", 3);
    ///
    /// let mut out = Vec::new();
    /// doc.write_to(&mut out)?;
    ///
    /// assert_eq!(
    ///     &out[..],
    ///     br#"
    ///     [first]
    ///     second = 2
    ///     third = 3
    ///     "#
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
        self.data.raw(self.root).display(&self.data, f)?;
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
