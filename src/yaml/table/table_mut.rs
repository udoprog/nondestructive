use crate::yaml::data::{Data, ValueId};
use crate::yaml::raw::{new_bool, new_string, RawKind, RawNumber};
use crate::yaml::serde;
use crate::yaml::{NullKind, Separator, Table, ValueMut};

/// Mutator for a table.
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let mut doc = yaml::from_bytes(r#"
///   number1: 10
///   number2: 20
///   table:
///     inner: 400
///   string3: "I am a quoted string!"
/// "#)?;
///
/// let mut root = doc.root_mut().into_table_mut().ok_or("missing root table")?;
///
/// assert_eq!(root.as_ref().get("number1").and_then(|v| v.as_u32()), Some(10));
/// assert_eq!(root.as_ref().get("number2").and_then(|v| v.as_u32()), Some(20));
/// assert_eq!(root.as_ref().get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
///
/// let table = root.get_mut("table").and_then(|v| v.into_table_mut()).ok_or("missing inner table")?;
/// assert_eq!(table.as_ref().get("inner").and_then(|v| v.as_u32()), Some(400));
///
/// root.get_mut("number2").ok_or("missing inner table")?.set_u32(30);
///
/// assert_eq!(
/// doc.to_string(),
/// r#"
///   number1: 10
///   number2: 30
///   table:
///     inner: 400
///   string3: "I am a quoted string!"
/// "#);
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
pub struct TableMut<'a> {
    data: &'a mut Data,
    id: ValueId,
}

macro_rules! insert_float {
    ($name:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_bytes(r#"
        ///   number1: 10
        /// "#)?;
        ///
        /// let mut value = doc.root_mut().into_table_mut().ok_or("not a table")?;
        #[doc = concat!("value.", stringify!($name), "(\"number2\", ", stringify!($lit), ");")]
        ///
        /// assert_eq!(
        /// doc.to_string(),
        /// r#"
        ///   number1: 10
        #[doc = concat!("  number2: ", stringify!($lit))]
        /// "#);
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, key: &str, value: $ty) {
            let mut buffer = ryu::Buffer::new();
            let number = self.data.insert_str(buffer.format(value));
            let value = RawKind::Number(RawNumber::new(number, serde::$hint));
            insert(self.data, self.id, key, Separator::Auto, value);
        }
    };
}

macro_rules! insert_number {
    ($name:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_bytes(r#"
        ///   number1: 10
        /// "#)?;
        /// let mut value = doc.root_mut().into_table_mut().ok_or("not a table")?;
        ///
        #[doc = concat!("value.", stringify!($name), "(\"number2\", ", stringify!($lit), ");")]
        ///
        /// assert_eq!(
        /// doc.to_string(),
        /// r#"
        ///   number1: 10
        #[doc = concat!("  number2: ", stringify!($lit))]
        /// "#);
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, key: &str, value: $ty) {
            let mut buffer = itoa::Buffer::new();
            let number = self.data.insert_str(buffer.format(value));
            let value = RawKind::Number(RawNumber::new(number, serde::$hint));
            insert(self.data, self.id, key, Separator::Auto, value);
        }
    };
}

impl<'a> TableMut<'a> {
    pub(crate) fn new(data: &'a mut Data, id: ValueId) -> Self {
        Self { data, id }
    }

    /// Coerce a mutable table as an immutable [Table].
    ///
    /// This is useful to be able to directly use methods only available on
    /// [Table].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
    ///   number1: 10
    ///   number2: 20
    ///   table:
    ///     inner: 400
    ///   string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut();
    /// let root = root.as_table_mut().ok_or("missing root table")?;
    /// let root = root.as_ref();
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    ///
    /// let table = root.get("table").and_then(|v| v.as_table()).ok_or("missing inner table")?;
    /// assert_eq!(table.get("inner").and_then(|v| v.as_u32()), Some(400));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn as_ref(&self) -> Table<'_> {
        Table::new(self.data, self.id)
    }

    /// Coerce a mutable table into an immutable [Table] with the lifetime of
    /// the current reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
    ///   number1: 10
    ///   number2: 20
    ///   table:
    ///     inner: 400
    ///   string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut();
    /// let root = root.as_table_mut().map(|t| t.into_ref()).ok_or("missing root table")?;
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    ///
    /// let table = root.get("table").and_then(|v| v.as_table()).ok_or("missing inner table")?;
    /// assert_eq!(table.get("inner").and_then(|v| v.as_u32()), Some(400));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn into_ref(self) -> Table<'a> {
        Table::new(self.data, self.id)
    }

    /// Get a value mutably from the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
    ///   number1: 10
    ///   number2: 20
    ///   table:
    ///     inner: 400
    ///   string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut();
    /// let mut root = root.as_table_mut().ok_or("missing root table")?;
    /// root.get_mut("number2").ok_or("missing inner table")?.set_u32(30);
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 30
    ///   table:
    ///     inner: 400
    ///   string3: "I am a quoted string!"
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_mut(&mut self, key: &str) -> Option<ValueMut<'_>> {
        for item in &self.data.table(self.id).items {
            if self.data.str(&item.key.string) == key {
                return Some(ValueMut::new(self.data, item.value));
            }
        }

        None
    }

    /// Insert a new null value and return a [`ValueMut`] to the newly inserted
    /// value.
    ///
    /// This allows for setting a custom [`Separator`].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     "#,
    /// )?;
    ///
    /// let mut root = doc.root_mut().into_table_mut().ok_or("missing root table")?;
    /// root.insert("three", yaml::Separator::Custom("   ")).set_u32(3);
    ///
    /// assert_eq! {
    ///     doc.to_string(),
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     three:   3
    ///     "#
    /// };
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn insert(&mut self, key: &str, separator: Separator<'_>) -> ValueMut<'_> {
        let value = insert(
            self.data,
            self.id,
            key,
            separator,
            RawKind::Null(NullKind::Empty),
        );
        ValueMut::new(self.data, value)
    }

    /// Insert a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
    ///   number1:  10
    /// "#)?;
    ///
    /// let mut value = doc.root_mut().into_table_mut().ok_or("not a table")?;
    /// value.insert_str("string2", "hello");
    ///
    /// assert_eq! (
    /// doc.to_string(),
    /// r#"
    ///   number1:  10
    ///   string2:  hello
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn insert_str<S>(&mut self, key: &str, string: S)
    where
        S: AsRef<str>,
    {
        let string = new_string(self.data, string);
        insert(self.data, self.id, key, Separator::Auto, string);
    }

    /// Insert a bool.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
    ///   number1: 10
    /// "#)?;
    /// let mut value = doc.root_mut().into_table_mut().ok_or("not a table")?;
    /// value.insert_bool("bool2", true);
    ///
    /// assert_eq! (
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   bool2: true
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn insert_bool(&mut self, key: &str, value: bool) {
        let value = new_bool(self.data, value);
        insert(self.data, self.id, key, Separator::Auto, value);
    }

    insert_float!(insert_f32, f32, "32-bit float", 10.42, F32);
    insert_float!(insert_f64, f64, "64-bit float", 10.42, F64);
    insert_number!(insert_u8, u8, "8-bit unsigned integer", 42, U8);
    insert_number!(insert_i8, i8, "8-bit signed integer", -42, I8);
    insert_number!(insert_u16, u16, "16-bit unsigned integer", 42, U16);
    insert_number!(insert_i16, i16, "16-bit signed integer", -42, I16);
    insert_number!(insert_u32, u32, "32-bit unsigned integer", 42, U32);
    insert_number!(insert_i32, i32, "32-bit signed integer", -42, I32);
    insert_number!(insert_u64, u64, "64-bit unsigned integer", 42, U64);
    insert_number!(insert_i64, i64, "64-bit signed integer", -42, I64);
    insert_number!(insert_u128, u128, "128-bit unsigned integer", 42, U128);
    insert_number!(insert_i128, i128, "128-bit signed integer", -42, I128);
}

/// Insert a value into the table.
pub(crate) fn insert(
    data: &mut Data,
    id: ValueId,
    key: &str,
    separator: Separator<'_>,
    value: RawKind,
) -> ValueId {
    use crate::yaml::raw::{Raw, RawString, RawStringKind, RawTableItem};

    let key = data.insert_str(key);

    if let Some(id) = data
        .table(id)
        .items
        .iter()
        .find(|c| c.key.string == key)
        .map(|item| item.value)
    {
        data.replace_raw(id, value);
        return id;
    }

    let key = RawString::new(RawStringKind::Bare, key);

    let separator = match separator {
        Separator::Auto => match data.table(id).items.last() {
            Some(last) => last.separator,
            None => data.insert_str(" "),
        },
        Separator::Custom(separator) => data.insert_str(separator),
    };

    let indent = data.layout(id).indent;
    let value = data.insert_raw(Raw::new(value, indent));
    let raw = data.table_mut(id);
    let prefix = (!raw.items.is_empty()).then_some(indent);

    raw.items.push(RawTableItem {
        prefix,
        key,
        separator,
        value,
    });

    value
}
