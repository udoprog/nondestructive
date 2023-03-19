use crate::yaml::data::{Data, ValueId};
use crate::yaml::raw::{new_bool, new_string, RawKind, RawNumber};
use crate::yaml::serde;
use crate::yaml::{AnyMut, MappingMut, NullKind, SequenceMut, Value};

/// A mutable value inside of a document.
pub struct ValueMut<'a> {
    data: &'a mut Data,
    pub(crate) id: ValueId,
}

impl<'a> ValueMut<'a> {
    /// Construct a new mutable value.
    pub(crate) fn new(data: &'a mut Data, id: ValueId) -> Self {
        Self { data, id }
    }

    /// Coerce into [`AnyMut`] to help discriminate the value type.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
    /// Hello World
    /// "#)?;
    ///
    /// assert!(matches!(doc.root_mut().into_any_mut(), yaml::AnyMut::Scalar(..)));
    ///
    /// let mut doc = yaml::from_bytes(r#"
    /// number1: 10
    /// number2: 20
    /// "#)?;
    ///
    /// assert!(matches!(doc.root_mut().into_any_mut(), yaml::AnyMut::Mapping(..)));
    ///
    /// let mut doc = yaml::from_bytes(r#"
    /// - 10
    /// - 20
    /// "#)?;
    ///
    /// assert!(matches!(doc.root_mut().into_any_mut(), yaml::AnyMut::Sequence(..)));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn into_any_mut(self) -> AnyMut<'a> {
        match &self.data.raw(self.id).kind {
            RawKind::Mapping(..) => AnyMut::Mapping(MappingMut::new(self.data, self.id)),
            RawKind::Sequence(..) => AnyMut::Sequence(SequenceMut::new(self.data, self.id)),
            _ => AnyMut::Scalar(self),
        }
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
    /// mapping:
    ///   inner: 400
    /// string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut().into_mapping_mut().ok_or("missing root mapping")?;
    ///
    /// assert_eq!(root.get_mut("number1").and_then(|v| v.as_ref().as_u32()), Some(10));
    /// assert_eq!(root.get_mut("number2").and_then(|v| v.as_ref().as_u32()), Some(20));
    ///
    /// let mut mapping = root.get_mut("mapping").and_then(|v| v.into_mapping_mut()).ok_or("missing inner mapping")?;
    /// assert_eq!(mapping.get_mut("inner").and_then(|v| v.as_ref().as_u32()), Some(400));
    ///
    /// assert_eq!(root.get_mut("string3").and_then(|v| v.into_ref().as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn as_ref(&self) -> Value<'_> {
        Value::new(self.data, self.id)
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
    /// mapping:
    ///   inner: 400
    /// string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut().into_mapping_mut().ok_or("missing root mapping")?;
    ///
    /// assert_eq!(root.get_mut("number1").and_then(|v| v.into_ref().as_u32()), Some(10));
    /// assert_eq!(root.get_mut("number2").and_then(|v| v.into_ref().as_u32()), Some(20));
    ///
    /// let mut mapping = root.get_mut("mapping").and_then(|v| v.into_mapping_mut()).ok_or("missing inner mapping")?;
    /// assert_eq!(mapping.get_mut("inner").and_then(|v| v.into_ref().as_u32()), Some(400));
    ///
    /// assert_eq!(root.get_mut("string3").and_then(|v| v.into_ref().as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn into_ref(self) -> Value<'a> {
        Value::new(self.data, self.id)
    }

    /// Convert the value into a mutable [`MappingMut`].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
    ///   number1: 10
    ///   number2: 20
    ///   mapping:
    ///     inner: 400
    ///   string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut();
    /// let mut root = root.as_mapping_mut().ok_or("missing root mapping")?;
    /// root.get_mut("number2").ok_or("missing inner mapping")?.set_u32(30);
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 30
    ///   mapping:
    ///     inner: 400
    ///   string3: "I am a quoted string!"
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn as_mapping_mut(&mut self) -> Option<MappingMut<'_>> {
        match &mut self.data.raw_mut(self.id).kind {
            RawKind::Mapping(..) => Some(MappingMut::new(self.data, self.id)),
            _ => None,
        }
    }

    /// Convert the value into a mutable [`MappingMut`] with the same lifetime as
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
    ///   mapping:
    ///     inner: 400
    ///   string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let mut root = doc.root_mut().into_mapping_mut().ok_or("missing root mapping")?;
    /// root.get_mut("number2").ok_or("missing inner mapping")?.set_u32(30);
    /// root.get_mut("string3").ok_or("missing inner mapping")?.set_string("i-am-a-bare-string");
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 30
    ///   mapping:
    ///     inner: 400
    ///   string3: i-am-a-bare-string
    /// "#);
    ///
    /// let mut root = doc.root_mut().into_mapping_mut().ok_or("missing root mapping")?;
    /// root.get_mut("string3").ok_or("missing inner mapping")?.set_string("It's \n a good day!");
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 30
    ///   mapping:
    ///     inner: 400
    ///   string3: "It's \n a good day!"
    /// "#);
    ///
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn into_mapping_mut(self) -> Option<MappingMut<'a>> {
        match &mut self.data.raw_mut(self.id).kind {
            RawKind::Mapping(..) => Some(MappingMut::new(self.data, self.id)),
            _ => None,
        }
    }

    /// Convert the value into a mutable [`SequenceMut`].
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
    /// let mut root = root.as_sequence_mut().ok_or("missing root sequence")?;
    /// root.get_mut(1).ok_or("missing inner mapping")?.set_u32(30);
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
    pub fn as_sequence_mut(&mut self) -> Option<SequenceMut<'_>> {
        match &mut self.data.raw_mut(self.id).kind {
            RawKind::Sequence(..) => Some(SequenceMut::new(self.data, self.id)),
            _ => None,
        }
    }

    /// Convert the value into a mutable [`SequenceMut`] with the same lifetime as
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
    /// let mut root = root.into_sequence_mut().ok_or("missing root sequence")?;
    /// root.get_mut(1).ok_or("missing inner mapping")?.set_u32(30);
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
    pub fn into_sequence_mut(self) -> Option<SequenceMut<'a>> {
        match &mut self.data.raw_mut(self.id).kind {
            RawKind::Sequence(..) => Some(SequenceMut::new(self.data, self.id)),
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
            let string = self.data.insert_str(buffer.format(value));
            self.data.replace_raw(self.id, RawKind::Number(RawNumber::new(string, serde::$hint)));
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
            let string = self.data.insert_str(buffer.format(value));
            self.data.replace_raw(self.id, RawKind::Number(RawNumber::new(string, serde::$hint)));
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
        self.data.replace_raw(self.id, RawKind::Null(kind));
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
    ///
    /// let mut doc = yaml::from_bytes("  string")?;
    /// doc.root_mut().set_string("null");
    /// assert_eq!(doc.to_string(), "  'null'");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn set_string<S>(&mut self, string: S)
    where
        S: AsRef<str>,
    {
        let value = new_string(self.data, string);
        self.data.replace_raw(self.id, value);
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
        let value = new_bool(value);
        self.data.replace_raw(self.id, value);
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
