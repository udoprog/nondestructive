use core::mem;

use crate::yaml::data::{Data, Id, StringId};
use crate::yaml::raw::{self, new_bool, new_string, Raw};
use crate::yaml::serde;
use crate::yaml::{Block, Mapping, Null, Separator, ValueMut};

/// Mutator for a mapping.
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
/// assert_eq!(root.as_ref().get("number1").and_then(|v| v.as_u32()), Some(10));
/// assert_eq!(root.as_ref().get("number2").and_then(|v| v.as_u32()), Some(20));
/// assert_eq!(root.as_ref().get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
///
/// let mapping = root.get_mut("mapping").and_then(|v| v.into_mapping_mut()).context("missing inner mapping")?;
/// assert_eq!(mapping.as_ref().get("inner").and_then(|v| v.as_u32()), Some(400));
///
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
pub struct MappingMut<'a> {
    data: &'a mut Data,
    pub(crate) id: Id,
}

macro_rules! insert_float {
    ($name:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
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
        ///     "#
        /// )?;
        ///
        /// let mut value = doc.as_mut().into_mapping_mut().context("not a mapping")?;
        #[doc = concat!("value.", stringify!($name), "(\"number2\", ", stringify!($lit), ");")]
        ///
        /// assert_eq!(
        ///     doc.to_string(),
        ///     r#"
        ///     number1: 10
        #[doc = concat!("    number2: ", stringify!($lit))]
        ///     "#
        /// );
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name<K>(&mut self, key: K, value: $ty)
        where
            K: AsRef<[u8]>,
        {
            let mut buffer = ryu::Buffer::new();
            let number = self.data.insert_str(buffer.format(value));
            let value = Raw::Number(raw::Number::new(number, serde::$hint));
            self._insert(key.as_ref(), Separator::Auto, value);
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
        /// use anyhow::Context;
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_slice(
        ///     r#"
        ///     number1: 10
        ///     "#
        /// )?;
        ///
        /// let mut value = doc.as_mut().into_mapping_mut().context("not a mapping")?;
        ///
        #[doc = concat!("value.", stringify!($name), "(\"number2\", ", stringify!($lit), ");")]
        ///
        /// assert_eq!(
        ///     doc.to_string(),
        ///     r#"
        ///     number1: 10
        #[doc = concat!("    number2: ", stringify!($lit))]
        ///     "#
        /// );
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name<K>(&mut self, key: K, value: $ty)
        where
            K: AsRef<[u8]>,
        {
            let mut buffer = itoa::Buffer::new();
            let number = self.data.insert_str(buffer.format(value));
            let value = Raw::Number(raw::Number::new(number, serde::$hint));
            self._insert(key.as_ref(), Separator::Auto, value);
        }
    };
}

impl<'a> MappingMut<'a> {
    pub(crate) fn new(data: &'a mut Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Make insertion prefix.
    fn make_prefix(&mut self) -> StringId {
        let mut out = Vec::new();
        out.push(raw::NEWLINE);
        out.resize(
            self.data.mapping(self.id).indent.saturating_add(1),
            raw::SPACE,
        );
        self.data.insert_str(out)
    }

    /// Insert a value into the mapping.
    fn _insert(&mut self, key: &[u8], separator: Separator<'_>, value: Raw) -> Id {
        let key = self.data.insert_str(key);

        if let Some(id) = self
            .data
            .mapping(self.id)
            .items
            .iter()
            .map(|id| self.data.mapping_item(*id))
            .find(|item| item.key.string == key)
            .map(|item| item.value)
        {
            self.data.replace(id, value);
            return id;
        }

        let key = raw::String::new(raw::RawStringKind::Bare, key);

        let item_prefix = if self.data.mapping(self.id).items.last().is_some() {
            self.make_prefix()
        } else {
            self.data.insert_str("")
        };

        let item_id = self
            .data
            .insert(Raw::Null(Null::Empty), item_prefix, Some(self.id));

        let value_prefix = match separator {
            Separator::Auto => match self.data.mapping(self.id).items.last() {
                Some(last) => self.data.layout(self.data.mapping_item(*last).value).prefix,
                None => self.data.insert_str(" "),
            },
            Separator::Custom(separator) => self.data.insert_str(separator),
        };

        let value = self.data.insert(value, value_prefix, Some(item_id));

        self.data
            .replace(item_id, Raw::MappingItem(raw::MappingItem { key, value }));
        self.data.mapping_mut(self.id).items.push(item_id);
        value
    }

    /// Coerce a mutable mapping as an immutable [Mapping].
    ///
    /// This is useful to be able to directly use methods only available on
    /// [Mapping].
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
    /// let root = root.as_mapping_mut().context("missing root mapping")?;
    /// let root = root.as_ref();
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    ///
    /// let mapping = root.get("mapping").and_then(|v| v.as_mapping()).context("missing inner mapping")?;
    /// assert_eq!(mapping.get("inner").and_then(|v| v.as_u32()), Some(400));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn as_ref(&self) -> Mapping<'_> {
        Mapping::new(self.data, self.id)
    }

    /// Coerce a mutable mapping into an immutable [Mapping] with the lifetime
    /// of the current reference.
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
    /// let root = root.as_mapping_mut().map(|m| m.into_ref()).context("missing root mapping")?;
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    ///
    /// let mapping = root.get("mapping").and_then(|v| v.as_mapping()).context("missing inner mapping")?;
    /// assert_eq!(mapping.get("inner").and_then(|v| v.as_u32()), Some(400));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn into_ref(self) -> Mapping<'a> {
        Mapping::new(self.data, self.id)
    }

    /// Get a value mutably from the mapping.
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
    pub fn get_mut(&mut self, key: &str) -> Option<ValueMut<'_>> {
        for item in &self.data.mapping(self.id).items {
            let item = self.data.mapping_item(*item);

            if self.data.str(item.key.string) == key {
                return Some(ValueMut::new(self.data, item.value));
            }
        }

        None
    }

    /// Get a value mutably from the mutable mapping with the lifetime of the
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
    ///     number1: 10
    ///     number2: 20
    ///     mapping:
    ///         inner: 400
    ///     string3: "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut();
    /// let mut value = root.as_mapping_mut().and_then(|v| v.get_into_mut("number2")).context("missing value")?;
    /// value.set_u32(30);
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
    #[must_use]
    pub fn get_into_mut(self, key: &str) -> Option<ValueMut<'a>> {
        for item in &self.data.mapping(self.id).items {
            let item = self.data.mapping_item(*item);

            if self.data.str(item.key.string) == key {
                return Some(ValueMut::new(self.data, item.value));
            }
        }

        None
    }

    /// Remove the given value from the mapping, returning a boolean indicating if
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
    ///
    /// assert!(!root.remove("no such key"));
    /// assert!(root.remove("mapping"));
    /// assert!(!root.remove("mapping"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     number1: 10
    ///     number2: 20
    ///     string3: "I am a quoted string!"
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn remove(&mut self, key: &str) -> bool {
        let mut index = None;

        for (i, item) in self.data.mapping(self.id).items.iter().enumerate() {
            let item = self.data.mapping_item(*item);

            if self.data.str(item.key.string) == key {
                index = Some(i);
                break;
            }
        }

        let Some(index) = index else {
            return false;
        };

        let item = self.data.mapping_mut(self.id).items.remove(index);
        self.data.drop(item);
        true
    }

    /// Clear all the elements in a mapping.
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
    ///
    /// root.clear();
    ///
    /// assert_eq!(doc.to_string(), "\n    \n    ");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn clear(&mut self) {
        let mut items = mem::take(&mut self.data.mapping_mut(self.id).items);

        for item in items.drain(..) {
            self.data.drop(item);
        }

        self.data.mapping_mut(self.id).items = items;
    }

    /// Insert a new null value and return a [`ValueMut`] to the newly inserted
    /// value.
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
    ///     one: 1
    ///     two: 2
    ///     "#,
    /// )?;
    ///
    /// let mut root = doc.as_mut().into_mapping_mut().context("missing root mapping")?;
    /// root.insert("three", yaml::Separator::Custom("   ")).set_u32(3);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     three:   3
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn insert<K>(&mut self, key: K, separator: Separator<'_>) -> ValueMut<'_>
    where
        K: AsRef<[u8]>,
    {
        let value = self._insert(key.as_ref(), separator, Raw::Null(Null::Empty));
        ValueMut::new(self.data, value)
    }

    /// Insert a string.
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
    ///     "#
    /// )?;
    ///
    /// let mut value = doc.as_mut().into_mapping_mut().context("not a mapping")?;
    /// value.insert_str("string2", "hello");
    ///
    /// assert_eq! (
    ///     doc.to_string(),
    ///     r#"
    ///     number1: 10
    ///     string2: hello
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn insert_str<K, S>(&mut self, key: K, string: S)
    where
        K: AsRef<[u8]>,
        S: AsRef<str>,
    {
        let string = new_string(self.data, string);
        self._insert(key.as_ref(), Separator::Auto, string);
    }

    /// Insert a value as a literal block.
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
    /// let mut sequence = doc.as_mut().make_mapping();
    /// sequence.clear();
    /// sequence.insert_block("key", ["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Clip));
    /// assert_eq!(sequence.as_ref().get("key").and_then(|v| v.as_str()), Some("foo\nbar\nbaz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     key: |
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_mapping();
    /// sequence.clear();
    /// sequence.insert_block("key", ["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Keep));
    /// assert_eq!(sequence.as_ref().get("key").and_then(|v| v.as_str()), Some("foo\nbar\nbaz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     key: |+
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_mapping();
    /// sequence.clear();
    /// sequence.insert_block("key", ["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Strip));
    /// assert_eq!(sequence.as_ref().get("key").and_then(|v| v.as_str()), Some("foo\nbar\nbaz"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     key: |-
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_mapping();
    /// sequence.clear();
    /// sequence.insert_block("key", ["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Clip));
    /// assert_eq!(sequence.as_ref().get("key").and_then(|v| v.as_str()), Some("foo bar baz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     key: >
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_mapping();
    /// sequence.clear();
    /// sequence.insert_block("key", ["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Keep));
    /// assert_eq!(sequence.as_ref().get("key").and_then(|v| v.as_str()), Some("foo bar baz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     key: >+
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_mapping();
    /// sequence.clear();
    /// sequence.insert_block("key", ["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Strip));
    /// assert_eq!(sequence.as_ref().get("key").and_then(|v| v.as_str()), Some("foo bar baz"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     key: >-
    ///       foo
    ///       bar
    ///       baz
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn insert_block<K, I>(&mut self, key: K, iter: I, block: Block)
    where
        K: AsRef<[u8]>,
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let value = raw::new_block(self.data, self.id, iter, block);
        self._insert(key.as_ref(), Separator::Auto, value);
    }

    /// Insert a bool.
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
    ///     "#
    /// )?;
    ///
    /// let mut value = doc.as_mut().into_mapping_mut().context("not a mapping")?;
    /// value.insert_bool("bool2", true);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     number1: 10
    ///     bool2: true
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn insert_bool<K>(&mut self, key: K, value: bool)
    where
        K: AsRef<[u8]>,
    {
        let value = new_bool(value);
        self._insert(key.as_ref(), Separator::Auto, value);
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
