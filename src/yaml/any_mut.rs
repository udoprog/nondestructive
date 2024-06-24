use crate::yaml::{Id, MappingMut, SequenceMut, ValueMut};

/// An enum which helps to externally discriminate the interior type of a
/// [`ValueMut`].
///
/// See [`ValueMut::into_any_mut`][crate::yaml::ValueMut::into_any_mut].
#[non_exhaustive]
pub enum AnyMut<'a> {
    /// The type is a scalar type.
    Scalar(ValueMut<'a>),
    /// The type is a [`MappingMut`].
    Mapping(MappingMut<'a>),
    /// The type is a [`SequenceMut`].
    Sequence(SequenceMut<'a>),
}

impl AnyMut<'_> {
    /// Get [`AnyMut`] identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - 10
    ///     - 20
    ///     "
    /// )?;
    ///
    /// let id = doc.as_mut().into_any_mut().id();
    ///
    /// assert_eq!(
    ///     doc.value(id).to_string(),
    ///     "- 10\n    - 20"
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn id(&self) -> Id {
        match self {
            AnyMut::Scalar(v) => v.id,
            AnyMut::Mapping(v) => v.id,
            AnyMut::Sequence(v) => v.id,
        }
    }
}
