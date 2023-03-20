use crate::yaml::{Id, Mapping, Sequence, Value};

/// An enum which helps to externally discriminate the interior type of a
/// [`Value`].
///
/// See [`Value::into_any`][crate::yaml::Value::into_any].
#[non_exhaustive]
pub enum Any<'a> {
    /// The type is a scalar type.
    Scalar(Value<'a>),
    /// The type is a [`Mapping`].
    Mapping(Mapping<'a>),
    /// The type is a [`Sequence`].
    Sequence(Sequence<'a>),
}

impl Any<'_> {
    /// Coerce into [`Any`] to help discriminate the value type.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice(
    ///     r#"
    ///     - 10
    ///     - 20
    ///     "#
    /// )?;
    ///
    /// let id = doc.as_ref().into_any().id();
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
            Any::Scalar(v) => v.id,
            Any::Mapping(v) => v.id,
            Any::Sequence(v) => v.id,
        }
    }
}
