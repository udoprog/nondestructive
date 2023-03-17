use crate::strings::Strings;
use crate::yaml::raw::{new_bool, new_string, Raw, RawKind, RawNumber};
use crate::yaml::serde;
use crate::yaml::{ListMut, NullKind, TableMut, Value};

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
    /// let mut doc = yaml::from_bytes(r#"
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
    #[must_use]
    #[inline]
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
    /// let mut doc = yaml::from_bytes(r#"
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
    #[must_use]
    #[inline]
    pub fn into_ref(self) -> Value<'a> {
        Value::new(self.strings, self.raw)
    }

    /// Convert the value into a mutable [`TableMut`].
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
    pub fn as_table_mut(&mut self) -> Option<TableMut<'_>> {
        match &mut self.raw.kind {
            RawKind::Table(raw) => Some(TableMut::new(self.strings, raw, &self.raw.layout)),
            _ => None,
        }
    }

    /// Convert the value into a mutable [`TableMut`] with the same lifetime as
    /// the one associated with this value.
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
    #[must_use]
    #[inline]
    pub fn into_table_mut(self) -> Option<TableMut<'a>> {
        match &mut self.raw.kind {
            RawKind::Table(raw) => Some(TableMut::new(self.strings, raw, &self.raw.layout)),
            _ => None,
        }
    }

    /// Convert the value into a mutable [`ListMut`].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
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

    /// Convert the value into a mutable [`ListMut`] with the same lifetime as
    /// the one associated with this value.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
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
    #[must_use]
    #[inline]
    pub fn into_list_mut(self) -> Option<ListMut<'a>> {
        match &mut self.raw.kind {
            RawKind::List(raw) => Some(ListMut::new(self.strings, raw, &self.raw.layout)),
            _ => None,
        }
    }
}

macro_rules! set_float {
    ($name:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_bytes("10")?;
        #[doc = concat!("let value = doc.root_mut().", stringify!($name), "(", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"", stringify!($lit), "\");")]
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = ryu::Buffer::new();
            let string = self.strings.insert(buffer.format(value));
            self.raw.kind = RawKind::Number(RawNumber::new(string, serde::$hint));
        }
    };
}

macro_rules! set_number {
    ($name:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_bytes("  10")?;
        #[doc = concat!("let value = doc.root_mut().", stringify!($name), "(", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"  ", stringify!($lit), "\");")]
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = itoa::Buffer::new();
            let string = self.strings.insert(buffer.format(value));
            self.raw.kind = RawKind::Number(RawNumber::new(string, serde::$hint));
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
    /// let mut doc = yaml::from_bytes("  string")?;
    /// doc.root_mut().set_null(yaml::NullKind::Keyword);
    /// assert_eq!(doc.to_string(), "  null");
    ///
    /// let mut doc = yaml::from_bytes("  string")?;
    /// doc.root_mut().set_null(yaml::NullKind::Tilde);
    /// assert_eq!(doc.to_string(), "  ~");
    ///
    /// let mut doc = yaml::from_bytes("  string")?;
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
    /// let mut doc = yaml::from_bytes("  string")?;
    /// doc.root_mut().set_string("i-am-a-string");
    /// assert_eq!(doc.to_string(), "  i-am-a-string");
    ///
    /// let mut doc = yaml::from_bytes("  string")?;
    /// doc.root_mut().set_string("I am a string");
    /// assert_eq!(doc.to_string(), "  I am a string");
    ///
    /// let mut doc = yaml::from_bytes("  string")?;
    /// doc.root_mut().set_string("I am a\n string");
    /// assert_eq!(doc.to_string(), "  \"I am a\\n string\"");
    ///
    /// let mut doc = yaml::from_bytes("  string")?;
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
    /// let mut doc = yaml::from_bytes("  string")?;
    /// doc.root_mut().set_bool(true);
    /// assert_eq!(doc.to_string(), "  true");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_bool(&mut self, value: bool) {
        let value = new_bool(self.strings, value);
        self.raw.kind = value;
    }

    set_float!(set_f32, f32, "32-bit float", 10.42, F32);
    set_float!(set_f64, f64, "64-bit float", 10.42, F64);
    set_number!(set_u8, u8, "8-bit unsigned integer", 42, U8);
    set_number!(set_i8, i8, "8-bit signed integer", -42, I8);
    set_number!(set_u16, u16, "16-bit unsigned integer", 42, U16);
    set_number!(set_i16, i16, "16-bit signed integer", -42, I16);
    set_number!(set_u32, u32, "32-bit unsigned integer", 42, U32);
    set_number!(set_i32, i32, "32-bit signed integer", -42, I32);
    set_number!(set_u64, u64, "64-bit unsigned integer", 42, U64);
    set_number!(set_i64, i64, "64-bit signed integer", -42, I64);
    set_number!(set_u128, u128, "128-bit unsigned integer", 42, U128);
    set_number!(set_i128, i128, "128-bit signed integer", -42, I128);
}
