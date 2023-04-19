use crate::toml::data::Data;
use crate::toml::{Id, Value};

/// Accessor for a table.
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use nondestructive::toml;
///
/// let doc = toml::from_slice(
///     r#"
///     number1 = 10
///     number2 = 20
///     string3 = "I am a quoted string!"
///
///     [table]
///     inner = 400
///     "#
/// )?;
///
/// let root = doc.as_ref();
///
/// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
/// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
///
/// let table = root.get("table").and_then(|v| v.as_table()).context("missing inner table")?;
/// assert_eq!(table.get("inner").and_then(|v| v.as_u32()), Some(400));
///
/// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
/// # Ok::<_, anyhow::Error>(())
/// ```
pub struct Table<'a> {
    data: &'a Data,
    id: Id,
}

impl<'a> Table<'a> {
    pub(crate) fn new(data: &'a Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Get a value from the table by its key.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::toml;
    ///
    /// let doc = toml::from_slice(
    ///     r#"
    ///     number1 = 10
    ///     number2 = 20
    ///     string3 = "I am a quoted string!"
    ///
    ///     [table]
    ///     inner = 400
    ///     "#
    /// )?;
    ///
    /// let root = doc.as_ref();
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    ///
    /// let table = root.get("table").and_then(|v| v.as_table()).context("missing inner table")?;
    /// assert_eq!(table.get("inner").and_then(|v| v.as_u32()), Some(400));
    ///
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn get(&self, key: &str) -> Option<Value<'a>> {
        for item in &self.data.table(self.id).items {
            let item = self.data.table_item(*item);

            if self.data.str(item.key.string) == key {
                return Some(Value::new(self.data, item.value));
            }
        }

        None
    }
}
