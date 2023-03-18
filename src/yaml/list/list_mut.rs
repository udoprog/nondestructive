use crate::yaml::data::Data;
use crate::yaml::raw::{new_bool, new_string, Layout, RawKind, RawList, RawNumber};
use crate::yaml::serde;
use crate::yaml::{List, NullKind, Separator, ValueMut};

/// Mutator for a list.
pub struct ListMut<'a> {
    data: &'a mut Data,
    raw: &'a mut RawList,
    layout: &'a Layout,
}

macro_rules! push_float {
    ($name:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Push the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_bytes(r#"
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
            let number = self.data.insert_str(buffer.format(value));
            let value = RawKind::Number(RawNumber::new(number, serde::$hint));
            self.raw
                .push(self.data, self.layout, Separator::Auto, value);
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
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_bytes(r#"
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
            let number = self.data.insert_str(buffer.format(value));
            let value = RawKind::Number(RawNumber::new(number, serde::$hint));
            self.raw
                .push(self.data, self.layout, Separator::Auto, value);
        }
    };
}

impl<'a> ListMut<'a> {
    pub(crate) fn new(data: &'a mut Data, raw: &'a mut RawList, layout: &'a Layout) -> Self {
        Self { data, raw, layout }
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
    /// let mut doc = yaml::from_bytes(
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
    #[must_use]
    #[inline]
    pub fn as_ref(&self) -> List<'_> {
        List::new(self.data, self.raw)
    }

    /// Coerce a mutable list into an immutable [List] with the lifetime of the
    /// current reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(
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
    #[must_use]
    #[inline]
    pub fn into_ref(self) -> List<'a> {
        List::new(self.data, self.raw)
    }

    /// Get a value mutably from the table.
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
    pub fn get_mut(&mut self, index: usize) -> Option<ValueMut<'_>> {
        if let Some(item) = self.raw.items.get_mut(index) {
            return Some(ValueMut::new(self.data, &mut item.value));
        }

        None
    }

    /// Push a new null value and return a [`ValueMut`] to the newly pushed value.
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
            self.data,
            self.layout,
            separator,
            RawKind::Null(NullKind::Empty),
        );
        // SAFETY: value was just pushed.
        let raw = unsafe { self.raw.items.get_unchecked_mut(index) };
        ValueMut::new(self.data, &mut raw.value)
    }

    /// Push a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
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
        let string = new_string(self.data, string);
        self.raw
            .push(self.data, self.layout, Separator::Auto, string);
    }

    /// Push a bool.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_bytes(r#"
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
        let value = new_bool(self.data, value);
        self.raw
            .push(self.data, self.layout, Separator::Auto, value);
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
