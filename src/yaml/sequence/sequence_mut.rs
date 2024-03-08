use core::mem;

use crate::yaml::data::{Data, Id, StringId};
use crate::yaml::raw::{self, Raw};
use crate::yaml::{Block, Null, Separator, Sequence, ValueMut};

/// Mutator for a sequence.
pub struct SequenceMut<'a> {
    data: &'a mut Data,
    pub(crate) id: Id,
}

macro_rules! push_float {
    ($name:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Push the value as a ", $string, ".")]
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
        ///     "#
        /// )?;
        ///
        /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
        ///
        #[doc = concat!("value.", stringify!($name), "(", stringify!($lit), ");")]
        /// assert_eq!(
        ///     doc.to_string(),
        ///     r#"
        ///     - 10
        #[doc = concat!("    - ", $lit)]
        ///     "#
        /// );
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = ryu::Buffer::new();
            let number = self.data.insert_str(buffer.format(value));
            let value = Raw::Number(raw::Number::new(number, crate::yaml::serde_hint::$hint));
            self._push(Separator::Auto, value);
        }
    };
}

macro_rules! push_number {
    ($name:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Push the value as a ", $string, ".")]
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
        ///     "#
        /// )?;
        ///
        /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
        ///
        #[doc = concat!("value.", stringify!($name), "(", stringify!($lit), ");")]
        ///
        /// assert_eq!(
        ///     doc.to_string(),
        ///     r#"
        ///     - 10
        #[doc = concat!("    - ", stringify!($lit))]
        ///     "#
        /// );
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = itoa::Buffer::new();
            let number = self.data.insert_str(buffer.format(value));
            let value = Raw::Number(raw::Number::new(number, crate::yaml::serde_hint::$hint));
            self._push(Separator::Auto, value);
        }
    };
}

impl<'a> SequenceMut<'a> {
    pub(crate) fn new(data: &'a mut Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Make insertion prefix.
    fn make_prefix(&mut self) -> StringId {
        let mut out = Vec::new();
        out.push(raw::NEWLINE);
        out.resize(
            self.data.sequence(self.id).indent.saturating_add(1),
            raw::SPACE,
        );
        self.data.insert_str(out)
    }

    /// Push a value on the sequence.
    fn _push(&mut self, separator: Separator, value: Raw) -> Id {
        let item_prefix = if self.data.sequence(self.id).items.last().is_some() {
            self.make_prefix()
        } else {
            self.data.insert_str("")
        };

        let item_id = self
            .data
            .insert(Raw::Null(Null::Empty), item_prefix, Some(self.id));

        let value_prefix = match separator {
            Separator::Auto => match self.data.sequence(self.id).items.last() {
                Some(last) => {
                    self.data
                        .layout(self.data.sequence_item(*last).value)
                        .prefix
                }
                None => self.data.insert_str(" "),
            },
            Separator::Custom(separator) => self.data.insert_str(separator),
        };

        let value = self.data.insert(value, value_prefix, Some(item_id));

        self.data
            .replace(item_id, Raw::SequenceItem(raw::SequenceItem { value }));
        self.data.sequence_mut(self.id).items.push(item_id);
        value
    }

    /// Coerce a mutable sequence as an immutable [Sequence].
    ///
    /// This is useful to be able to directly use methods only available on
    /// [Sequence].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_mut().into_sequence_mut().context("missing root sequence")?;
    /// let root = root.as_ref();
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn as_ref(&self) -> Sequence<'_> {
        Sequence::new(self.data, self.id)
    }

    /// Coerce a mutable sequence into an immutable [Sequence] with the lifetime of the
    /// current reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.as_mut().into_sequence_mut().context("missing root sequence")?.into_ref();
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn into_ref(self) -> Sequence<'a> {
        Sequence::new(self.data, self.id)
    }

    /// Get a value mutably from the sequence.
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
    /// root.get_mut(1).context("missing inner sequence")?.set_u32(30);
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
    pub fn get_mut(&mut self, index: usize) -> Option<ValueMut<'_>> {
        if let Some(item) = self.data.sequence(self.id).items.get(index) {
            let item = self.data.sequence_item(*item);
            return Some(ValueMut::new(self.data, item.value));
        }

        None
    }

    /// Get a value mutably from the sequence.
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
    /// let mut value = root.as_sequence_mut().and_then(|v| v.get_into_mut(1)).context("missing value")?;
    /// value.set_u32(30);
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
    pub fn get_into_mut(self, index: usize) -> Option<ValueMut<'a>> {
        if let Some(item) = self.data.sequence(self.id).items.get(index) {
            let item = self.data.sequence_item(*item);
            return Some(ValueMut::new(self.data, item.value));
        }

        None
    }

    /// Remove the given index from the sequence, returning a boolean indicating if
    /// it existed in the sequence or not.
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
    ///
    /// assert!(!root.remove(4));
    /// assert!(root.remove(2));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - 10
    ///     - 20
    ///     - "I am a quoted string!"
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn remove(&mut self, index: usize) -> bool {
        let raw = self.data.sequence_mut(self.id);

        if raw.items.len() <= index {
            return false;
        }

        let item = raw.items.remove(index);
        self.data.drop(item);
        true
    }

    /// Clear all the elements in a sequence.
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
    ///
    /// root.clear();
    ///
    /// assert_eq!(doc.to_string(), "\n    \n    ");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn clear(&mut self) {
        let mut items = mem::take(&mut self.data.sequence_mut(self.id).items);

        for item in items.drain(..) {
            self.data.drop(item);
        }

        self.data.sequence_mut(self.id).items = items;
    }

    /// Push a new null value and return a [`ValueMut`] to the newly pushed value.
    ///
    /// This allows for setting a custom [`Separator`].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - one
    ///     - two
    ///     "#,
    /// )?;
    ///
    /// let mut root = doc.as_mut().into_sequence_mut().context("missing root sequence")?;
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
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn push(&mut self, separator: Separator<'_>) -> ValueMut<'_> {
        let value = self._push(separator, Raw::Null(Null::Empty));
        ValueMut::new(self.data, value)
    }

    /// Push a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - - 10
    ///     "#
    /// )?;
    ///
    /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    /// let mut value = value.get_mut(0).and_then(|v| v.into_sequence_mut()).expect("missing inner");
    /// value.push_string("nice string");
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - - 10
    ///       - nice string
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn push_string<S>(&mut self, string: S)
    where
        S: AsRef<str>,
    {
        let string = raw::new_string(self.data, string);
        self._push(Separator::Auto, string);
    }

    /// Push a value as a literal block.
    ///
    /// This takes an iterator, which will be used to construct the block. The
    /// underlying value type produced is in fact a string, and can be read
    /// through methods such as [`Value::as_str`][crate::yaml::Value::as_str].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     string
    ///     "#
    /// )?;
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Clip));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo\nbar\nbaz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - |
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Keep));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo\nbar\nbaz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - |+
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Strip));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo\nbar\nbaz"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - |-
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Clip));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo bar baz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - >
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Keep));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo bar baz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - >+
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Strip));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo bar baz"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - >-
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn push_block<I>(&mut self, iter: I, block: Block)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let value = raw::new_block(self.data, self.id, iter, block);
        self._push(Separator::Auto, value);
    }

    /// Push a bool.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - - 10
    ///     "#
    /// )?;
    ///
    /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    /// let mut value = value.get_mut(0).and_then(|v| v.into_sequence_mut()).expect("missing inner");
    /// value.push_bool(false);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - - 10
    ///       - false
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn push_bool(&mut self, value: bool) {
        let value = raw::new_bool(value);
        self._push(Separator::Auto, value);
    }

    push_float!(push_f32, f32, "32-bit float", 10.42, F32);
    push_float!(push_f64, f64, "64-bit float", 10.42, F64);
    push_number!(push_u8, u8, "8-bit unsigned integer", 42, U8);
    push_number!(push_i8, i8, "8-bit signed integer", -42, I8);
    push_number!(push_u16, u16, "16-bit unsigned integer", 42, U16);
    push_number!(push_i16, i16, "16-bit signed integer", -42, I16);
    push_number!(push_u32, u32, "32-bit unsigned integer", 42, U32);
    push_number!(push_i32, i32, "32-bit signed integer", -42, I32);
    push_number!(push_u64, u64, "64-bit unsigned integer", 42, U64);
    push_number!(push_i64, i64, "64-bit signed integer", -42, I64);
    push_number!(push_u128, u128, "128-bit unsigned integer", 42, U128);
    push_number!(push_i128, i128, "128-bit signed integer", -42, I128);
}
