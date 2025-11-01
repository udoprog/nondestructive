use alloc::vec::Vec;

use crate::yaml::data::{Data, Id};
use crate::yaml::raw::{self, Raw};
use crate::yaml::{AnyMut, Block, MappingMut, Null, SequenceMut, StringKind, Value};

/// A mutable value inside of a document.
pub struct ValueMut<'a> {
    data: &'a mut Data,
    pub(crate) id: Id,
}

impl<'a> ValueMut<'a> {
    /// Construct a new mutable value.
    pub(crate) fn new(data: &'a mut Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Coerce into [`AnyMut`] to help discriminate the value type.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     Hello World
    ///     "
    /// )?;
    ///
    /// assert!(matches!(doc.as_mut().into_any_mut(), yaml::AnyMut::Scalar(..)));
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     number1: 10
    ///     number2: 20
    ///     "
    /// )?;
    ///
    /// assert!(matches!(doc.as_mut().into_any_mut(), yaml::AnyMut::Mapping(..)));
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - 10
    ///     - 20
    ///     "
    /// )?;
    ///
    /// assert!(matches!(doc.as_mut().into_any_mut(), yaml::AnyMut::Sequence(..)));
    /// # Ok::<_, anyhow::Error>(())
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
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     number1: 10
    ///     number2: 20
    ///     mapping:
    ///         inner: 400
    ///     string3: "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut().into_mapping_mut().context("missing root mapping")?;
    ///
    /// assert_eq!(root.get_mut("number1").and_then(|v| v.as_ref().as_u32()), Some(10));
    /// assert_eq!(root.get_mut("number2").and_then(|v| v.as_ref().as_u32()), Some(20));
    ///
    /// let mut mapping = root.get_mut("mapping").and_then(|v| v.into_mapping_mut()).context("missing inner mapping")?;
    /// assert_eq!(mapping.get_mut("inner").and_then(|v| v.as_ref().as_u32()), Some(400));
    ///
    /// assert_eq!(root.get_mut("string3").and_then(|v| v.into_ref().as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, anyhow::Error>(())
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
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     number1: 10
    ///     number2: 20
    ///     mapping:
    ///         inner: 400
    ///     string3: "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut().into_mapping_mut().context("missing root mapping")?;
    ///
    /// assert_eq!(root.get_mut("number1").and_then(|v| v.into_ref().as_u32()), Some(10));
    /// assert_eq!(root.get_mut("number2").and_then(|v| v.into_ref().as_u32()), Some(20));
    ///
    /// let mut mapping = root.get_mut("mapping").and_then(|v| v.into_mapping_mut()).context("missing inner mapping")?;
    /// assert_eq!(mapping.get_mut("inner").and_then(|v| v.into_ref().as_u32()), Some(400));
    ///
    /// assert_eq!(root.get_mut("string3").and_then(|v| v.into_ref().as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, anyhow::Error>(())
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
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     number1: 10
    ///     number2: 20
    ///     mapping:
    ///         inner: 400
    ///     string3: "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut();
    /// let mut root = root.as_mapping_mut().context("missing root mapping")?;
    /// root.get_mut("number2").context("missing number2")?.set_u32(30);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     number1: 10
    ///     number2: 30
    ///     mapping:
    ///         inner: 400
    ///     string3: "I am a quoted string!"
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
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
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     number1: 10
    ///     number2: 20
    ///     mapping:
    ///         inner: 400
    ///     string3: "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut().into_mapping_mut().context("missing root mapping")?;
    /// root.get_mut("number2").context("missing number2")?.set_u32(30);
    /// root.get_mut("string3").context("missing string3")?.set_string("i-am-a-bare-string");
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     number1: 10
    ///     number2: 30
    ///     mapping:
    ///         inner: 400
    ///     string3: i-am-a-bare-string
    ///     "
    /// );
    ///
    /// let mut root = doc.as_mut().into_mapping_mut().context("missing root mapping")?;
    /// root.get_mut("string3").context("missing string3")?.set_string("It's \n a good day!");
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     number1: 10
    ///     number2: 30
    ///     mapping:
    ///         inner: 400
    ///     string3: "It's \n a good day!"
    ///     "#
    /// );
    ///
    /// # Ok::<_, anyhow::Error>(())
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
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - 10
    ///     - 20
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut();
    /// let mut root = root.as_sequence_mut().context("missing root sequence")?;
    /// root.get_mut(1).context("missing inner mapping")?.set_u32(30);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - 10
    ///     - 30
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
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
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - 10
    ///     - 20
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut();
    /// let mut root = root.into_sequence_mut().context("missing root sequence")?;
    /// root.get_mut(1).context("missing inner mapping")?.set_u32(30);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - 10
    ///     - 30
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
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
        /// use anyhow::Context;
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_slice("10")?;
        ///
        #[doc = concat!("let value = doc.as_mut().", stringify!($name), "(", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"", stringify!($lit), "\");")]
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = ryu::Buffer::new();
            let string = self.data.insert_str(buffer.format(value));
            self.data.replace(self.id, Raw::Number(raw::Number::new(string, crate::yaml::serde_hint::$hint)));
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
        /// use anyhow::Context;
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_slice("  10")?;
        ///
        #[doc = concat!("let value = doc.as_mut().", stringify!($name), "(", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"  ", stringify!($lit), "\");")]
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = itoa::Buffer::new();
            let string = self.data.insert_str(buffer.format(value));
            self.data.replace(self.id, Raw::Number(raw::Number::new(string, crate::yaml::serde_hint::$hint)));
        }
    };
}

impl<'a> ValueMut<'a> {
    /// Replace the current value with the specified null value.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice("  string")?;
    ///
    /// doc.as_mut().set_null(yaml::Null::Keyword);
    /// assert_eq!(doc.to_string(), "  null");
    ///
    /// doc.as_mut().set_null(yaml::Null::Tilde);
    /// assert_eq!(doc.to_string(), "  ~");
    ///
    /// doc.as_mut().set_null(yaml::Null::Empty);
    /// assert_eq!(doc.to_string(), "  ");
    ///
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[inline]
    pub fn set_null(&mut self, kind: Null) {
        let raw_kind = match kind {
            Null::Keyword => raw::Null::Keyword(self.data.insert_str("null")),
            Null::Tilde => raw::Null::Tilde,
            Null::Empty => raw::Null::Empty,
        };
        self.data.replace(self.id, Raw::Null(raw_kind));
    }

    /// Set the value as a string.
    ///
    /// The [`StringKind`] used will follow a fairly simple heuristic documented
    /// below, if this is not suitable then [`ValueMut::set_string_with`] may be
    /// used.
    ///
    /// The heuristic used is:
    /// * [`StringKind::Single`] will be used if the leading digit is numeric.
    /// * [`StringKind::Double`] will be used if either a single `'` is
    ///   encounted, or a non-graphical component that requires escaping.
    /// * Otherwise, [`StringKind::Bare`] is used.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice("  string")?;
    ///
    /// doc.as_mut().set_string("i-am-a-string");
    /// assert_eq!(doc.to_string(), "  i-am-a-string");
    ///
    /// doc.as_mut().set_string("I am a string");
    /// assert_eq!(doc.to_string(), "  I am a string");
    ///
    /// doc.as_mut().set_string("I am a\n string");
    /// assert_eq!(doc.to_string(), "  \"I am a\\n string\"");
    ///
    /// doc.as_mut().set_string("I am a string with \"quotes\"");
    /// assert_eq!(doc.to_string(), "  I am a string with \"quotes\"");
    ///
    /// doc.as_mut().set_string("null");
    /// assert_eq!(doc.to_string(), "  'null'");
    ///
    /// doc.as_mut().set_string("1.65");
    /// assert_eq!(doc.to_string(), "  '1.65'");
    ///
    /// doc.as_mut().set_string("rust@1.65");
    /// assert_eq!(doc.to_string(), "  rust@1.65");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[inline]
    pub fn set_string<S>(&mut self, string: S)
    where
        S: AsRef<str>,
    {
        let value = raw::new_string(self.data, string);
        self.data.replace(self.id, value);
    }

    /// Set the value as a string with a custom [`StringKind`].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice("  string")?;
    ///
    /// doc.as_mut().set_string_with("i-am-a-string", yaml::StringKind::Double);
    /// assert_eq!(doc.to_string(), "  \"i-am-a-string\"");
    ///
    /// doc.as_mut().set_string_with("It's a great success!", yaml::StringKind::Single);
    /// assert_eq!(doc.to_string(), "  'It''s a great success!'");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[inline]
    pub fn set_string_with<S>(&mut self, string: S, kind: StringKind)
    where
        S: AsRef<str>,
    {
        let value = raw::new_string_with(self.data, string, kind);
        self.data.replace(self.id, value);
    }

    /// Set the value as a literal block.
    ///
    /// This takes an iterator, which will be used to construct the block. The
    /// underlying value type produced is in fact a string, and can be read
    /// through methods such as [`Value::as_str`].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     string
    ///     "
    /// )?;
    ///
    /// doc.as_mut().set_block(["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Clip));
    /// assert_eq!(doc.as_ref().as_str(), Some("foo\nbar\nbaz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     |
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// doc.as_mut().set_block(["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Keep));
    /// assert_eq!(doc.as_ref().as_str(), Some("foo\nbar\nbaz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     |+
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// doc.as_mut().set_block(["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Strip));
    /// assert_eq!(doc.as_ref().as_str(), Some("foo\nbar\nbaz"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     |-
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// doc.as_mut().set_block(["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Clip));
    /// assert_eq!(doc.as_ref().as_str(), Some("foo bar baz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     >
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// doc.as_mut().set_block(["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Keep));
    /// assert_eq!(doc.as_ref().as_str(), Some("foo bar baz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     >+
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// doc.as_mut().set_block(["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Strip));
    /// assert_eq!(doc.as_ref().as_str(), Some("foo bar baz"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     >-
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn set_block<I>(&mut self, iter: I, block: Block)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let value = raw::new_block(self.data, self.id, iter, block);
        self.data.replace(self.id, value);
    }

    /// Set the value as a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice("  string")?;
    ///
    /// doc.as_mut().set_bool(true);
    /// assert_eq!(doc.to_string(), "  true");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn set_bool(&mut self, value: bool) {
        let value = raw::new_bool(self.data, value);
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
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     string
    ///     "
    /// )?;
    ///
    /// let mut mapping = doc.as_mut().make_mapping();
    /// mapping.insert_u32("first", 1);
    /// mapping.insert_u32("second", 2);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     first: 1
    ///     second: 2
    ///     "
    /// );
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     first: second
    ///     "
    /// )?;
    ///
    /// let mut mapping = doc.as_mut().into_mapping_mut().and_then(|m| Some(m.get_into_mut("first")?.make_mapping())).context("missing first")?;
    /// mapping.insert_u32("second", 2);
    /// mapping.insert_u32("third", 3);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     first:
    ///       second: 2
    ///       third: 3
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    ///
    /// Making a new mapping inside of a sequence item:
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - one
    ///     - two
    ///     ",
    /// )?;
    ///
    /// let mut seq = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    /// let mut mapping = seq.push(yaml::Separator::Auto).make_mapping();
    ///
    /// mapping.insert_u32("three", 3);
    /// mapping.insert_u32("four", 4);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - one
    ///     - two
    ///     - three: 3
    ///       four: 4
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn make_mapping(self) -> MappingMut<'a> {
        if !matches!(self.data.raw(self.id), Raw::Mapping(..)) {
            let (indent, prefix) = raw::make_indent(self.data, self.id);

            self.data.replace_with(
                self.id,
                prefix,
                Raw::Mapping(raw::Mapping {
                    indent,
                    kind: raw::MappingKind::Mapping,
                    items: Vec::new(),
                }),
            );
        }

        MappingMut::new(self.data, self.id)
    }

    /// Make the value into a sequence, unless it already is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     string
    ///     "
    /// )?;
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.push_u32(1);
    /// sequence.push_u32(2);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - 1
    ///     - 2
    ///     "
    /// );
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     first: second
    ///     "
    /// )?;
    ///
    /// let mut sequence = doc.as_mut().into_mapping_mut().and_then(|m| Some(m.get_into_mut("first")?.make_sequence())).context("missing first")?;
    /// sequence.push_u32(2);
    /// sequence.push_u32(3);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     first:
    ///       - 2
    ///       - 3
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn make_sequence(self) -> SequenceMut<'a> {
        if !matches!(self.data.raw(self.id), Raw::Sequence(..)) {
            let (indent, prefix) = raw::make_indent(self.data, self.id);

            self.data.replace_with(
                self.id,
                prefix,
                Raw::Sequence(raw::Sequence {
                    indent,
                    kind: raw::SequenceKind::Mapping,
                    items: Vec::new(),
                }),
            );
        }

        SequenceMut::new(self.data, self.id)
    }
}
