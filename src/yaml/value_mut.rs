use crate::slab::Pointer;
use crate::yaml::raw::{Raw, RawNumber};
use crate::yaml::raw::{RawString, RawTable};
use crate::yaml::{Document, NullKind, StringKind, Table, Value};

/// A mutable value inside of a document.
pub struct ValueMut<'a> {
    doc: &'a mut Document,
    pointer: Pointer,
}

impl<'a> ValueMut<'a> {
    /// Construct a new mutable value.
    pub(crate) fn new(doc: &'a mut Document, pointer: Pointer) -> Self {
        Self { doc, pointer }
    }

    /// Get a raw element based on the current pointer.
    pub(crate) fn raw(&self) -> Option<&Raw> {
        self.doc.tree.get(&self.pointer)
    }

    /// Get a mutable value as an immutable [Value].
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
        Value::new(self.doc, self.pointer)
    }

    /// Convert a mutable value as an immutable [Value] with the lifetime of the
    /// mutable reference.
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
        Value::new(self.doc, self.pointer)
    }

    /// Get the value as a mutable table.
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
    /// assert_eq! {
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 30
    ///   table:
    ///     inner: 400
    ///   string3: "I am a quoted string!"
    /// "#
    /// };
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn as_table_mut(&mut self) -> Option<TableMut<'_>> {
        match self.raw() {
            Some(Raw::Table(..)) => Some(TableMut::new(self.doc, self.pointer)),
            _ => None,
        }
    }

    /// Convert the value into a mutable table with the same lifetime as the one
    /// associated with this [ValueMut].
    ///
    /// This is useful when dealing with inline values.
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
    /// assert_eq! {
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 30
    ///   table:
    ///     inner: 400
    ///   string3: i-am-a-bare-string
    /// "#
    /// };
    ///
    /// let mut root = doc.root_mut().into_table_mut().ok_or("missing root table")?;
    /// root.get_mut("string3").ok_or("missing inner table")?.set_string("It's \n a good day!");
    ///
    /// assert_eq! {
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 30
    ///   table:
    ///     inner: 400
    ///   string3: "It's \n a good day!"
    /// "#
    /// };
    ///
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn into_table_mut(self) -> Option<TableMut<'a>> {
        match self.raw() {
            Some(Raw::Table(..)) => Some(TableMut::new(self.doc, self.pointer)),
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
            if let Some(raw) = self.doc.tree.get_mut(&self.pointer) {
                let mut buffer = ryu::Buffer::new();
                let string = self.doc.strings.insert(buffer.format(value));
                *raw = Raw::Number(RawNumber::new(string));
            }
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
        /// let mut doc = yaml::parse("10")?;
        #[doc = concat!("let value = doc.root_mut().", stringify!($name), "(", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"", stringify!($lit), "\");")]
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            if let Some(raw) = self.doc.tree.get_mut(&self.pointer) {
                let mut buffer = itoa::Buffer::new();
                let string = self.doc.strings.insert(buffer.format(value));
                *raw = Raw::Number(RawNumber::new(string));
            }
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
    pub fn set_null(&mut self, kind: NullKind) {
        if let Some(raw) = self.doc.tree.get_mut(&self.pointer) {
            *raw = Raw::Null(kind);
        }
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
    pub fn set_string<S>(&mut self, string: S)
    where
        S: AsRef<str>,
    {
        let Some(raw) = self.doc.tree.get_mut(&self.pointer) else {
            return;
        };

        let kind = StringKind::detect(string.as_ref());
        let string = self.doc.strings.insert(string.as_ref());
        let string = RawString::new(kind, string);
        *raw = Raw::String(string);
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
        let value = self.doc.insert_bool(value);

        let Some(raw) = self.doc.tree.get_mut(&self.pointer) else {
            return;
        };

        *raw = value;
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

/// Accessor for a table.
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
/// assert_eq! {
/// doc.to_string(),
/// r#"
///   number1: 10
///   number2: 30
///   table:
///     inner: 400
///   string3: "I am a quoted string!"
/// "#
/// };
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
pub struct TableMut<'a> {
    doc: &'a mut Document,
    pointer: Pointer,
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
        /// number1: 10
        /// "#)?;
        /// let mut value = doc.root_mut().into_table_mut().ok_or("not a table")?;
        #[doc = concat!("value.", stringify!($name), "(\"number2\", ", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"\\nnumber1: 10\\nnumber2: ", stringify!($lit), "\\n\");")]
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, key: &str, value: $ty) {
            if !self.doc.tree.contains(&self.pointer, |m| matches!(m, Raw::Table(..))) {
                return;
            }

            let mut buffer = ryu::Buffer::new();
            let number = self.doc.strings.insert(buffer.format(value));
            let value = self.doc.tree.insert(Raw::Number(RawNumber::new(number)));

            if let Some(Raw::Table(table)) = self.doc.tree.get_mut(&self.pointer) {
                let separator = self.doc.strings.insert(" ");
                let kind = StringKind::detect(key);
                let key = self.doc.strings.insert(key);
                let key = RawString::new(kind, key);
                table.insert(key, separator, value);
            }
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
        /// number1: 10
        /// "#)?;
        /// let mut value = doc.root_mut().into_table_mut().ok_or("not a table")?;
        #[doc = concat!("value.", stringify!($name), "(\"number2\", ", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"\\nnumber1: 10\\nnumber2: ", stringify!($lit), "\\n\");")]
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, key: &str, value: $ty) {
            if !self.doc.tree.contains(&self.pointer, |m| matches!(m, Raw::Table(..))) {
                return;
            }

            let mut buffer = itoa::Buffer::new();
            let number = self.doc.strings.insert(buffer.format(value));
            let value = self.doc.tree.insert(Raw::Number(RawNumber::new(number)));

            if let Some(Raw::Table(table)) = self.doc.tree.get_mut(&self.pointer) {
                let separator = self.doc.strings.insert(" ");
                let kind = StringKind::detect(key);
                let key = self.doc.strings.insert(key);
                let key = RawString::new(kind, key);
                table.insert(key, separator, value);
            }
        }
    };
}

impl<'a> TableMut<'a> {
    pub(crate) fn new(doc: &'a mut Document, pointer: Pointer) -> Self {
        Self { doc, pointer }
    }

    /// Get the raw element based on the value pointer.
    pub(crate) fn raw(&self) -> Option<&RawTable> {
        match self.doc.tree.get(&self.pointer) {
            Some(Raw::Table(table)) => Some(table),
            _ => None,
        }
    }

    /// Get a mutable table as an immutable [Table].
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
        Table::new(self.doc, self.pointer)
    }

    /// Get a mutable table as an immutable [Table].
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
        Table::new(self.doc, self.pointer)
    }

    /// Get a value mutably from the table.
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
    /// let mut root = doc.root_mut();
    /// let mut root = root.as_table_mut().ok_or("missing root table")?;
    /// root.get_mut("number2").ok_or("missing inner table")?.set_u32(30);
    ///
    /// assert_eq!(doc.to_string(), "\nnumber1: 10\nnumber2: 30\ntable:\n  inner: 400\nstring3: \"I am a quoted string!\"\n");
    ///
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_mut(&mut self, key: &str) -> Option<ValueMut<'_>> {
        let raw = self.raw()?;

        for e in &raw.items {
            if self.doc.strings.get(&e.key.string) == key {
                return Some(ValueMut::new(self.doc, e.value));
            }
        }

        None
    }

    /// Insert a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    /// number1: 10
    /// "#)?;
    /// let mut value = doc.root_mut().into_table_mut().ok_or("not a table")?;
    /// value.insert_string("string2", "hello");
    /// assert_eq!(doc.to_string(), "\nnumber1: 10\nstring2: hello\n");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn insert_string<S>(&mut self, key: &str, string: S)
    where
        S: AsRef<str>,
    {
        if !self
            .doc
            .tree
            .contains(&self.pointer, |m| matches!(m, Raw::Table(..)))
        {
            return;
        }

        let kind = StringKind::detect(string.as_ref());
        let string = self.doc.strings.insert(string.as_ref());
        let string = Raw::String(RawString::new(kind, string));
        let string = self.doc.tree.insert(string);

        if let Some(Raw::Table(table)) = self.doc.tree.get_mut(&self.pointer) {
            let separator = self.doc.strings.insert(" ");
            let kind = StringKind::detect(key);
            let key = self.doc.strings.insert(key);
            let key = RawString::new(kind, key);
            table.insert(key, separator, string);
        }
    }

    /// Insert a bool.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::parse(r#"
    /// number1: 10
    /// "#)?;
    /// let mut value = doc.root_mut().into_table_mut().ok_or("not a table")?;
    /// value.insert_bool("bool2", true);
    /// assert_eq!(doc.to_string(), "\nnumber1: 10\nbool2: true\n");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn insert_bool(&mut self, key: &str, value: bool) {
        if !self
            .doc
            .tree
            .contains(&self.pointer, |m| matches!(m, Raw::Table(..)))
        {
            return;
        }

        let value = self.doc.insert_bool(value);
        let value = self.doc.tree.insert(value);

        if let Some(Raw::Table(table)) = self.doc.tree.get_mut(&self.pointer) {
            let separator = self.doc.strings.insert(" ");
            let kind = StringKind::detect(key);
            let key = self.doc.strings.insert(key);
            let key = RawString::new(kind, key);
            table.insert(key, separator, value);
        }
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
