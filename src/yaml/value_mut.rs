use bstr::ByteSlice;

use crate::yaml::data::{Data, ValueId};
use crate::yaml::raw::{
    make_mapping, make_sequence, new_bool, new_string, Layout, Raw, RawNumber, NEWLINE,
};
use crate::yaml::serde;
use crate::yaml::{AnyMut, MappingMut, NullKind, SequenceMut, Value};

use super::data::StringId;

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
    #[must_use]
    pub fn into_any_mut(self) -> AnyMut<'a> {
        match self.data.raw(self.id) {
            Raw::Mapping(..) => AnyMut::Mapping(MappingMut::new(self.data, self.id)),
            Raw::Sequence(..) => AnyMut::Sequence(SequenceMut::new(self.data, self.id)),
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
        match self.data.raw_mut(self.id) {
            Raw::Mapping(..) => Some(MappingMut::new(self.data, self.id)),
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
    pub fn into_mapping_mut(self) -> Option<MappingMut<'a>> {
        match self.data.raw_mut(self.id) {
            Raw::Mapping(..) => Some(MappingMut::new(self.data, self.id)),
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
        match self.data.raw_mut(self.id) {
            Raw::Sequence(..) => Some(SequenceMut::new(self.data, self.id)),
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
    pub fn into_sequence_mut(self) -> Option<SequenceMut<'a>> {
        match self.data.raw_mut(self.id) {
            Raw::Sequence(..) => Some(SequenceMut::new(self.data, self.id)),
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
            self.data.replace(self.id, Raw::Number(RawNumber::new(string, serde::$hint)));
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
            self.data.replace(self.id, Raw::Number(RawNumber::new(string, serde::$hint)));
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
        self.data.replace(self.id, Raw::Null(kind));
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
        self.data.replace(self.id, value);
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
        self.data.replace(self.id, value);
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

    /// Make the value into a mapping, unless it already is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes("  string")?;
    /// let mut mapping = doc.root_mut().make_mapping();
    /// mapping.insert_u32("first", 1);
    /// mapping.insert_u32("second", 2);
    ///
    /// assert_eq!(doc.to_string(), "  first: 1\n  second: 2");
    ///
    /// let mut doc = yaml::from_bytes(
    ///     r#"
    ///     first: second
    ///     "#
    /// )?;
    /// let mut mapping = doc.root_mut().into_mapping_mut().and_then(|m| Some(m.get_into_mut("first")?.make_mapping())).ok_or("missing first")?;
    /// mapping.insert_u32("second", 2);
    /// mapping.insert_u32("third", 3);
    ///
    /// // TODO: support this
    /// // assert_eq!(
    /// //     doc.to_string(),
    /// //     r#"
    /// //     first:
    /// //         second: 2
    /// //         third: 3
    /// //     "#
    /// // );
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn make_mapping(mut self) -> MappingMut<'a> {
        if !matches!(self.data.raw(self.id), Raw::Mapping(..)) {
            let indent = self.build_indent();
            let value = make_mapping();
            self.data.replace_with_indent(self.id, value, indent);
        }

        MappingMut::new(self.data, self.id)
    }

    /// Make the value into a sequence, unless it already is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes("  string")?;
    /// let mut mapping = doc.root_mut().make_sequence();
    /// mapping.push_u32(1);
    /// mapping.push_u32(2);
    ///
    /// assert_eq!(doc.to_string(), "  - 1\n  - 2");
    ///
    /// let mut doc = yaml::from_bytes(
    ///     r#"
    ///     first: second
    ///     "#
    /// )?;
    /// let mut mapping = doc.root_mut().into_mapping_mut().and_then(|m| Some(m.get_into_mut("first")?.make_sequence())).ok_or("missing first")?;
    /// mapping.push_u32(2);
    /// mapping.push_u32(3);
    ///
    /// // TODO: support this
    /// // assert_eq!(
    /// //     doc.to_string(),
    /// //     r#"
    /// //     first:
    /// //       - 2
    /// //       - 3
    /// //     "#
    /// // );
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn make_sequence(mut self) -> SequenceMut<'a> {
        if !matches!(self.data.raw(self.id), Raw::Sequence(..)) {
            let indent = self.build_indent();
            let value = make_sequence();
            self.data.replace_with_indent(self.id, value, indent);
        }

        SequenceMut::new(self.data, self.id)
    }

    /// Make indentation for mappings and sequences.
    ///
    /// This is a bit of a handsful, but the gist is that we need to calculate
    /// the new indentation to use for elements in this mapping.
    ///
    /// If we have a parent node, we extend the indentation of the parent node,
    /// else we take the indentation from the current node, ensuring that it
    /// starts with a newline.
    fn build_indent(&mut self) -> StringId {
        let mut new_indent = Vec::new();
        new_indent.push(NEWLINE);

        match *self.data.layout(self.id) {
            Layout {
                parent: Some(id), ..
            } => {
                let indent = self.data.layout(id).prefix;
                new_indent.extend_from_slice(self.data.str(indent));
                new_indent.extend_from_slice(b"  ");
            }
            Layout {
                prefix: indent,
                parent: None,
            } => {
                let string = self.data.str(indent);
                let string = match string.as_bytes() {
                    [NEWLINE, rest @ ..] => rest,
                    string => string,
                };
                new_indent.extend_from_slice(string);
            }
        };

        self.data.insert_str(new_indent)
    }
}
