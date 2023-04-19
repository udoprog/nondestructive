use crate::toml::data::{Data, Id};
use crate::toml::ValueMut;

/// Mutator for a table.
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use nondestructive::toml;
///
/// let mut doc = toml::from_slice(
///     r#"
///     number1 = 10
///     number2 = 20
///     string3 = "I am a quoted string!"
///     [table]
///     inner = 400
///     "#
/// )?;
///
/// let mut root = doc.as_mut();
///
/// assert_eq!(root.as_ref().get("number1").and_then(|v| v.as_u32()), Some(10));
/// assert_eq!(root.as_ref().get("number2").and_then(|v| v.as_u32()), Some(20));
/// assert_eq!(root.as_ref().get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
///
/// let table = root.get_mut("table").and_then(|v| v.into_table_mut()).context("missing inner table")?;
/// assert_eq!(table.as_ref().get("inner").and_then(|v| v.as_u32()), Some(400));
///
/// root.get_mut("number2").context("missing number2")?.set_u32(30);
///
/// assert_eq!(
///     doc.to_string(),
///     r#"
///     number1 = 10
///     number2 = 30
///     string3 = "I am a quoted string!"
///     [table]
///     inner = 400
///     "#
/// );
/// # Ok::<_, anyhow::Error>(())
/// ```
pub struct TableMut<'a> {
    data: &'a mut Data,
    pub(crate) id: Id,
}

impl<'a> TableMut<'a> {
    #[inline]
    pub(crate) fn new(data: &'a mut Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Get a value mutably from the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     number1 = 10
    ///     number2 = 20
    ///     string3 = "I am a quoted string!"
    ///
    ///     [mapping]
    ///     inner = 400
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut();
    /// root.get_mut("number2").context("missing number2")?.set_u32(30);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     number1 = 10
    ///     number2 = 30
    ///     string3 = "I am a quoted string!"
    ///
    ///     [table]
    ///     inner = 400
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn get_mut(&mut self, key: &str) -> Option<ValueMut<'_>> {
        for item in &self.data.table(self.id).items {
            let item = self.data.table_item(*item);

            if self.data.str(item.key.string) == key {
                return Some(ValueMut::new(self.data, item.value));
            }
        }

        None
    }
}
