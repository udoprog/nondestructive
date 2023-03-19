use core::mem;

use crate::yaml::data::{Data, ValueId};
use crate::yaml::raw::{new_bool, new_string, RawKind, RawNumber};
use crate::yaml::serde;
use crate::yaml::{Mapping, NullKind, Separator, ValueMut};

/// Mutator for a mapping.
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
///
/// assert_eq!(root.as_ref().get("number1").and_then(|v| v.as_u32()), Some(10));
/// assert_eq!(root.as_ref().get("number2").and_then(|v| v.as_u32()), Some(20));
/// assert_eq!(root.as_ref().get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
///
/// let mapping = root.get_mut("mapping").and_then(|v| v.into_mapping_mut()).ok_or("missing inner mapping")?;
/// assert_eq!(mapping.as_ref().get("inner").and_then(|v| v.as_u32()), Some(400));
///
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
pub struct MappingMut<'a> {
    data: &'a mut Data,
    pub(crate) id: ValueId,
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
        /// let mut value = doc.root_mut().into_mapping_mut().ok_or("not a mapping")?;
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
            self._insert(key, Separator::Auto, value);
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
        /// let mut value = doc.root_mut().into_mapping_mut().ok_or("not a mapping")?;
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
            self._insert(key, Separator::Auto, value);
        }
    };
}

impl<'a> MappingMut<'a> {
    pub(crate) fn new(data: &'a mut Data, id: ValueId) -> Self {
        Self { data, id }
    }

    /// Insert a value into the mapping.
    fn _insert(&mut self, key: &str, separator: Separator<'_>, value: RawKind) -> ValueId {
        use crate::yaml::raw::{Raw, RawMappingItem, RawString, RawStringKind};

        let key = self.data.insert_str(key);

        if let Some(id) = self
            .data
            .mapping(self.id)
            .items
            .iter()
            .find(|c| c.key.string == key)
            .map(|item| item.value)
        {
            self.data.replace_raw(id, value);
            return id;
        }

        let key = RawString::new(RawStringKind::Bare, key);

        let separator = match separator {
            Separator::Auto => match self.data.mapping(self.id).items.last() {
                Some(last) => last.separator,
                None => self.data.insert_str(" "),
            },
            Separator::Custom(separator) => self.data.insert_str(separator),
        };

        let indent = self.data.layout(self.id).indent;
        let value = self.data.insert_raw(Raw::new(value, indent));
        let raw = self.data.mapping_mut(self.id);
        let prefix = (!raw.items.is_empty()).then_some(indent);

        raw.items.push(RawMappingItem {
            prefix,
            key,
            separator,
            value,
        });

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
    /// let root = root.as_mapping_mut().ok_or("missing root mapping")?;
    /// let root = root.as_ref();
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    ///
    /// let mapping = root.get("mapping").and_then(|v| v.as_mapping()).ok_or("missing inner mapping")?;
    /// assert_eq!(mapping.get("inner").and_then(|v| v.as_u32()), Some(400));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
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
    /// let root = root.as_mapping_mut().map(|t| t.into_ref()).ok_or("missing root mapping")?;
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    ///
    /// let mapping = root.get("mapping").and_then(|v| v.as_mapping()).ok_or("missing inner mapping")?;
    /// assert_eq!(mapping.get("inner").and_then(|v| v.as_u32()), Some(400));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
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
    pub fn get_mut(&mut self, key: &str) -> Option<ValueMut<'_>> {
        for item in &self.data.mapping(self.id).items {
            if self.data.str(&item.key.string) == key {
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
    /// let mut value = root.as_mapping_mut().and_then(|v| v.get_into_mut("number2")).ok_or("missing value")?;
    /// value.set_u32(30);
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
    pub fn get_into_mut(self, key: &str) -> Option<ValueMut<'a>> {
        for item in &self.data.mapping(self.id).items {
            if self.data.str(&item.key.string) == key {
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
    ///
    /// assert!(!root.remove("no such key"));
    /// assert!(root.remove("mapping"));
    /// assert!(!root.remove("mapping"));
    ///
    /// assert_eq!(
    /// doc.to_string(),
    /// r#"
    ///   number1: 10
    ///   number2: 20
    ///   string3: "I am a quoted string!"
    /// "#);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove(&mut self, key: &str) -> bool {
        let mut index = None;

        for (i, item) in self.data.mapping(self.id).items.iter().enumerate() {
            if self.data.str(&item.key.string) == key {
                index = Some(i);
                break;
            }
        }

        let Some(index) = index else {
            return false;
        };

        let item = self.data.mapping_mut(self.id).items.remove(index);
        self.data.drop(item.value);
        true
    }

    /// Clear all the elements in a mapping.
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
    ///
    /// root.clear();
    ///
    /// assert_eq!(doc.to_string(), "\n  \n");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn clear(&mut self) {
        let mut items = mem::take(&mut self.data.mapping_mut(self.id).items);

        for item in items.drain(..) {
            self.data.drop(item.value);
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
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(
    ///     r#"
    ///     one: 1
    ///     two: 2
    ///     "#,
    /// )?;
    ///
    /// let mut root = doc.root_mut().into_mapping_mut().ok_or("missing root mapping")?;
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
        let value = self._insert(key, separator, RawKind::Null(NullKind::Empty));
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
    /// let mut value = doc.root_mut().into_mapping_mut().ok_or("not a mapping")?;
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
        self._insert(key, Separator::Auto, string);
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
    /// let mut value = doc.root_mut().into_mapping_mut().ok_or("not a mapping")?;
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
        let value = new_bool(value);
        self._insert(key, Separator::Auto, value);
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
