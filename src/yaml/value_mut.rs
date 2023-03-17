use crate::strings::Strings;
use crate::yaml::raw::{Layout, Raw, RawKind, RawList, RawNumber, RawTable};
use crate::yaml::{List, NullKind, Separator, Table, Value};

use super::raw::{new_bool, new_string};

/// A mutable value inside of a document.
pub struct ValueMut<'a> {
    strings: &'a mut Strings,
    raw: &'a mut Raw,
}

impl<'a> ValueMut<'a> {
    /// Construct a new mutable value.
    pub(crate) fn new(strings: &'a mut Strings, raw: &'a mut Raw) -> Self {
        Self { strings, raw }
    }

    /// Coerce a mutable value as an immutable [Value].
    ///
    /// This is useful to be able to directly use methods only available on
    /// [Value].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    /// number1: 10
    /// number2: 20
    /// table:
    ///   inner: 400
    /// string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut().into_table_mut().ok_or("missing root table")?;
    ///
    /// assert_eq!(root.get_mut("number1").and_then(|v| v.as_ref().as_u32()), Some(10));
    /// assert_eq!(root.get_mut("number2").and_then(|v| v.as_ref().as_u32()), Some(20));
    ///
    /// let mut table = root.get_mut("table").and_then(|v| v.into_table_mut()).ok_or("missing inner table")?;
    /// assert_eq!(table.get_mut("inner").and_then(|v| v.as_ref().as_u32()), Some(400));
    ///
    /// assert_eq!(root.get_mut("string3").and_then(|v| v.into_ref().as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn as_ref(&self) -> Value<'_> {
        Value::new(self.strings, self.raw)
    }

    /// Coerce a mutable value into an immutable [Value] with the lifetime of
    /// the current reference.
    ///
    /// This is useful to be able to directly use methods only available on
    /// [Value].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    /// number1: 10
    /// number2: 20
    /// table:
    ///   inner: 400
    /// string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut().into_table_mut().ok_or("missing root table")?;
    ///
    /// assert_eq!(root.get_mut("number1").and_then(|v| v.into_ref().as_u32()), Some(10));
    /// assert_eq!(root.get_mut("number2").and_then(|v| v.into_ref().as_u32()), Some(20));
    ///
    /// let mut table = root.get_mut("table").and_then(|v| v.into_table_mut()).ok_or("missing inner table")?;
    /// assert_eq!(table.get_mut("inner").and_then(|v| v.into_ref().as_u32()), Some(400));
    ///
    /// assert_eq!(root.get_mut("string3").and_then(|v| v.into_ref().as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn into_ref(self) -> Value<'a> {
        Value::new(self.strings, self.raw)
    }

    /// Convert the value into a mutable [Table].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
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
    pub fn as_table_mut(&mut self) -> Option<TableMut<'_>> {
        match &mut self.raw.kind {
            RawKind::Table(raw) => Some(TableMut::new(self.strings, raw, &self.raw.layout)),
            _ => None,
        }
    }

    /// Convert the value into a mutable [Table] with the same lifetime as the
    /// one associated with this value.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    ///   number1: 10
    ///   number2: 20
    ///   table:
    ///     inner: 400
    ///   string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut().into_table_mut().ok_or("missing root table")?;
    /// root.get_mut("number2").ok_or("missing inner table")?.set_u32(30);
    /// root.get_mut("string3").ok_or("missing inner table")?.set_string("i-am-a-bare-string");
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 30
    ///   table:
    ///     inner: 400
    ///   string3: i-am-a-bare-string
    /// "#);
    ///
    /// let mut root = doc.root_mut().into_table_mut().ok_or("missing root table")?;
    /// root.get_mut("string3").ok_or("missing inner table")?.set_string("It's \n a good day!");
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 30
    ///   table:
    ///     inner: 400
    ///   string3: "It's \n a good day!"
    /// "#);
    ///
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn into_table_mut(self) -> Option<TableMut<'a>> {
        match &mut self.raw.kind {
            RawKind::Table(raw) => Some(TableMut::new(self.strings, raw, &self.raw.layout)),
            _ => None,
        }
    }

    /// Convert the value into a mutable [List].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    ///   - 10
    ///   - 20
    ///   - inner: 400
    ///   - "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut();
    /// let mut root = root.as_list_mut().ok_or("missing root list")?;
    /// root.get_mut(1).ok_or("missing inner table")?.set_u32(30);
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   - 10
    ///   - 30
    ///   - inner: 400
    ///   - "I am a quoted string!"
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn as_list_mut(&mut self) -> Option<ListMut<'_>> {
        match &mut self.raw.kind {
            RawKind::List(raw) => Some(ListMut::new(self.strings, raw, &self.raw.layout)),
            _ => None,
        }
    }

    /// Convert the value into a mutable [List] with the same lifetime as the
    /// one associated with this value.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    ///   - 10
    ///   - 20
    ///   - inner: 400
    ///   - "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut();
    /// let mut root = root.into_list_mut().ok_or("missing root list")?;
    /// root.get_mut(1).ok_or("missing inner table")?.set_u32(30);
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   - 10
    ///   - 30
    ///   - inner: 400
    ///   - "I am a quoted string!"
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn into_list_mut(self) -> Option<ListMut<'a>> {
        match &mut self.raw.kind {
            RawKind::List(raw) => Some(ListMut::new(self.strings, raw, &self.raw.layout)),
            _ => None,
        }
    }
}

macro_rules! set_float {
    ($name:ident, $ty:ty, $string:literal, $lit:literal) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::parse("10")?;
        #[doc = concat!("let value = doc.root_mut().", stringify!($name), "(", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"", stringify!($lit), "\");")]
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = ryu::Buffer::new();
            let string = self.strings.insert(buffer.format(value));
            self.raw.kind = RawKind::Number(RawNumber::new(string));
        }
    };
}

macro_rules! set_number {
    ($name:ident, $ty:ty, $string:literal, $lit:literal) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::parse("  10")?;
        #[doc = concat!("let value = doc.root_mut().", stringify!($name), "(", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"  ", stringify!($lit), "\");")]
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = itoa::Buffer::new();
            let string = self.strings.insert(buffer.format(value));
            self.raw.kind = RawKind::Number(RawNumber::new(string));
        }
    };
}

impl<'a> ValueMut<'a> {
    /// Replace the current value with the specified null value.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse("  string")?;
    /// doc.root_mut().set_null(yaml::NullKind::Keyword);
    /// assert_eq!(doc.to_string(), "  null");
    ///
    /// let mut doc = yaml::parse("  string")?;
    /// doc.root_mut().set_null(yaml::NullKind::Tilde);
    /// assert_eq!(doc.to_string(), "  ~");
    ///
    /// let mut doc = yaml::parse("  string")?;
    /// doc.root_mut().set_null(yaml::NullKind::Empty);
    /// assert_eq!(doc.to_string(), "  ");
    ///
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn set_null(&mut self, kind: NullKind) {
        self.raw.kind = RawKind::Null(kind);
    }

    /// Set the value as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse("  string")?;
    /// doc.root_mut().set_string("i-am-a-string");
    /// assert_eq!(doc.to_string(), "  i-am-a-string");
    ///
    /// let mut doc = yaml::parse("  string")?;
    /// doc.root_mut().set_string("I am a string");
    /// assert_eq!(doc.to_string(), "  I am a string");
    ///
    /// let mut doc = yaml::parse("  string")?;
    /// doc.root_mut().set_string("I am a\n string");
    /// assert_eq!(doc.to_string(), "  \"I am a\\n string\"");
    ///
    /// let mut doc = yaml::parse("  string")?;
    /// doc.root_mut().set_string("I am a string with \"quotes\"");
    /// assert_eq!(doc.to_string(), "  I am a string with \"quotes\"");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn set_string<S>(&mut self, string: S)
    where
        S: AsRef<str>,
    {
        self.raw.kind = new_string(self.strings, string);
    }

    /// Set the value as a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse("  string")?;
    /// doc.root_mut().set_bool(true);
    /// assert_eq!(doc.to_string(), "  true");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_bool(&mut self, value: bool) {
        let value = new_bool(self.strings, value);
        self.raw.kind = value;
    }

    set_float!(set_f32, f32, "32-bit float", 10.42);
    set_float!(set_f64, f64, "64-bit float", 10.42);
    set_number!(set_u8, u8, "8-bit unsigned integer", 42);
    set_number!(set_i8, i8, "8-bit signed integer", -42);
    set_number!(set_u16, u16, "16-bit unsigned integer", 42);
    set_number!(set_i16, i16, "16-bit signed integer", -42);
    set_number!(set_u32, u32, "32-bit unsigned integer", 42);
    set_number!(set_i32, i32, "32-bit signed integer", -42);
    set_number!(set_u64, u64, "64-bit unsigned integer", 42);
    set_number!(set_i64, i64, "64-bit signed integer", -42);
    set_number!(set_u128, u128, "128-bit unsigned integer", 42);
    set_number!(set_i128, i128, "128-bit signed integer", -42);
}

/// Mutator for a table.
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let mut doc = yaml::parse(r#"
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
    strings: &'a mut Strings,
    raw: &'a mut RawTable,
    layout: &'a Layout,
}

macro_rules! insert_float {
    ($name:ident, $ty:ty, $string:literal, $lit:literal) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::parse(r#"
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
            let number = self.strings.insert(buffer.format(value));
            let value = RawKind::Number(RawNumber::new(number));
            self.raw
                .insert(self.strings, self.layout, key, Separator::Auto, value);
        }
    };
}

macro_rules! insert_number {
    ($name:ident, $ty:ty, $string:literal, $lit:literal) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::parse(r#"
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
            let number = self.strings.insert(buffer.format(value));
            let value = RawKind::Number(RawNumber::new(number));
            self.raw
                .insert(self.strings, self.layout, key, Separator::Auto, value);
        }
    };
}

impl<'a> TableMut<'a> {
    pub(crate) fn new(strings: &'a mut Strings, raw: &'a mut RawTable, layout: &'a Layout) -> Self {
        Self {
            strings,
            raw,
            layout,
        }
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
    /// let mut doc = yaml::parse(r#"
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
    pub fn as_ref(&self) -> Table<'_> {
        Table::new(self.strings, self.raw)
    }

    /// Coerce a mutable table into an immutable [Table] with the lifetime of
    /// the current reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
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
    pub fn into_ref(self) -> Table<'a> {
        Table::new(self.strings, self.raw)
    }

    /// Get a value mutably from the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
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
        for e in &mut self.raw.items {
            if self.strings.get(&e.key.string) == key {
                return Some(ValueMut::new(self.strings, &mut e.value));
            }
        }

        None
    }

    /// Insert a new null value and return a [ValueMut] to the newly inserted
    /// value.
    ///
    /// This allows for setting a custom [Separator].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(
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
        let index = self.raw.insert(
            self.strings,
            self.layout,
            key,
            separator,
            RawKind::Null(NullKind::Empty),
        );
        // SAFETY: value was just inserted.
        let raw = unsafe { self.raw.items.get_unchecked_mut(index) };
        ValueMut::new(self.strings, &mut raw.value)
    }

    /// Insert a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    ///   number1:  10
    /// "#)?;
    ///
    /// let mut value = doc.root_mut().into_table_mut().ok_or("not a table")?;
    /// value.insert_string("string2", "hello");
    ///
    /// assert_eq! (
    /// doc.to_string(),
    /// r#"
    ///   number1:  10
    ///   string2:  hello
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn insert_string<S>(&mut self, key: &str, string: S)
    where
        S: AsRef<str>,
    {
        let string = new_string(self.strings, string);
        self.raw
            .insert(self.strings, self.layout, key, Separator::Auto, string);
    }

    /// Insert a bool.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
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
        let value = new_bool(self.strings, value);
        self.raw
            .insert(self.strings, self.layout, key, Separator::Auto, value);
    }

    insert_float!(insert_f32, f32, "32-bit float", 10.42);
    insert_float!(insert_f64, f64, "64-bit float", 10.42);
    insert_number!(insert_u8, u8, "8-bit unsigned integer", 42);
    insert_number!(insert_i8, i8, "8-bit signed integer", -42);
    insert_number!(insert_u16, u16, "16-bit unsigned integer", 42);
    insert_number!(insert_i16, i16, "16-bit signed integer", -42);
    insert_number!(insert_u32, u32, "32-bit unsigned integer", 42);
    insert_number!(insert_i32, i32, "32-bit signed integer", -42);
    insert_number!(insert_u64, u64, "64-bit unsigned integer", 42);
    insert_number!(insert_i64, i64, "64-bit signed integer", -42);
    insert_number!(insert_u128, u128, "128-bit unsigned integer", 42);
    insert_number!(insert_i128, i128, "128-bit signed integer", -42);
}

/// Mutator for a list.
pub struct ListMut<'a> {
    strings: &'a mut Strings,
    raw: &'a mut RawList,
    layout: &'a Layout,
}

macro_rules! push_float {
    ($name:ident, $ty:ty, $string:literal, $lit:literal) => {
        #[doc = concat!("Push the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::parse(r#"
        /// - 10
        /// "#)?;
        ///
        /// let mut value = doc.root_mut().into_list_mut().ok_or("not a list")?;
        ///
        #[doc = concat!("value.", stringify!($name), "(", stringify!($lit), ");")]
        /// assert_eq!(
        /// doc.to_string(),
        /// r#"
        /// - 10
        #[doc = concat!("- ", $lit)]
        /// "#);
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = ryu::Buffer::new();
            let number = self.strings.insert(buffer.format(value));
            let value = RawKind::Number(RawNumber::new(number));
            self.raw
                .push(self.strings, self.layout, Separator::Auto, value);
        }
    };
}

macro_rules! push_number {
    ($name:ident, $ty:ty, $string:literal, $lit:literal) => {
        #[doc = concat!("Push the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::parse(r#"
        /// - 10
        /// "#)?;
        /// let mut value = doc.root_mut().into_list_mut().ok_or("not a list")?;
        ///
        #[doc = concat!("value.", stringify!($name), "(", stringify!($lit), ");")]
        ///
        /// assert_eq!(
        /// doc.to_string(),
        /// r#"
        /// - 10
        #[doc = concat!("- ", stringify!($lit))]
        /// "#);
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = itoa::Buffer::new();
            let number = self.strings.insert(buffer.format(value));
            let value = RawKind::Number(RawNumber::new(number));
            self.raw
                .push(self.strings, self.layout, Separator::Auto, value);
        }
    };
}

impl<'a> ListMut<'a> {
    pub(crate) fn new(strings: &'a mut Strings, raw: &'a mut RawList, layout: &'a Layout) -> Self {
        Self {
            strings,
            raw,
            layout,
        }
    }

    /// Coerce a mutable list as an immutable [List].
    ///
    /// This is useful to be able to directly use methods only available on
    /// [List].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(
    /// r#"
    /// - one
    /// - two
    /// - three
    /// "#,
    /// )?;
    ///
    /// let root = doc.root_mut().into_list_mut().ok_or("missing root list")?;
    /// let root = root.as_ref();
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn as_ref(&self) -> List<'_> {
        List::new(self.strings, self.raw)
    }

    /// Coerce a mutable list into an immutable [List] with the lifetime of the
    /// current reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(
    /// r#"
    /// - one
    /// - two
    /// - three
    /// "#,
    /// )?;
    ///
    /// let root = doc.root_mut().into_list_mut().ok_or("missing root list")?.into_ref();
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn into_ref(self) -> List<'a> {
        List::new(self.strings, self.raw)
    }

    /// Get a value mutably from the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    ///   - 10
    ///   - 20
    ///   - inner: 400
    ///   - "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut();
    /// let mut root = root.as_list_mut().ok_or("missing root list")?;
    /// root.get_mut(1).ok_or("missing inner table")?.set_u32(30);
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   - 10
    ///   - 30
    ///   - inner: 400
    ///   - "I am a quoted string!"
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_mut(&mut self, index: usize) -> Option<ValueMut<'_>> {
        if let Some(item) = self.raw.items.get_mut(index) {
            return Some(ValueMut::new(self.strings, &mut item.value));
        }

        None
    }

    /// Push a new null value and return a [ValueMut] to the newly pushed value.
    ///
    /// This allows for setting a custom [Separator].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(
    ///     r#"
    ///     - one
    ///     - two
    ///     "#,
    /// )?;
    ///
    /// let mut root = doc.root_mut().into_list_mut().ok_or("missing root list")?;
    /// root.push(yaml::Separator::Custom("   ")).set_bool(true);
    ///
    /// assert_eq! {
    ///     doc.to_string(),
    ///     r#"
    ///     - one
    ///     - two
    ///     -   true
    ///     "#
    /// };
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn push(&mut self, separator: Separator<'_>) -> ValueMut<'_> {
        let index = self.raw.items.len();
        self.raw.push(
            self.strings,
            self.layout,
            separator,
            RawKind::Null(NullKind::Empty),
        );
        // SAFETY: value was just pushed.
        let raw = unsafe { self.raw.items.get_unchecked_mut(index) };
        ValueMut::new(self.strings, &mut raw.value)
    }

    /// Push a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    ///   - - 10
    /// "#)?;
    /// let mut value = doc.root_mut().into_list_mut().ok_or("not a list")?;
    /// let mut value = value.get_mut(0).and_then(|v| v.into_list_mut()).expect("missing inner");
    /// value.push_string("nice string");
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   - - 10
    ///     - nice string
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn push_string<S>(&mut self, string: S)
    where
        S: AsRef<str>,
    {
        let string = new_string(self.strings, string);
        self.raw
            .push(self.strings, self.layout, Separator::Auto, string);
    }

    /// Push a bool.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    ///   - - 10
    /// "#)?;
    /// let mut value = doc.root_mut().into_list_mut().ok_or("not a list")?;
    /// let mut value = value.get_mut(0).and_then(|v| v.into_list_mut()).expect("missing inner");
    /// value.push_bool(false);
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   - - 10
    ///     - false
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn push_bool(&mut self, value: bool) {
        let value = new_bool(self.strings, value);
        self.raw
            .push(self.strings, self.layout, Separator::Auto, value);
    }

    push_float!(push_f32, f32, "32-bit float", 10.42);
    push_float!(push_f64, f64, "64-bit float", 10.42);
    push_number!(push_u8, u8, "8-bit unsigned integer", 42);
    push_number!(push_i8, i8, "8-bit signed integer", -42);
    push_number!(push_u16, u16, "16-bit unsigned integer", 42);
    push_number!(push_i16, i16, "16-bit signed integer", -42);
    push_number!(push_u32, u32, "32-bit unsigned integer", 42);
    push_number!(push_i32, i32, "32-bit signed integer", -42);
    push_number!(push_u64, u64, "64-bit unsigned integer", 42);
    push_number!(push_i64, i64, "64-bit signed integer", -42);
    push_number!(push_u128, u128, "128-bit unsigned integer", 42);
    push_number!(push_i128, i128, "128-bit signed integer", -42);
}
