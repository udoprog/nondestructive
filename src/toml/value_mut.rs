use crate::toml::data::{Data, Id};
use crate::toml::raw::{self, Raw};

/// A mutable value inside of a document.
pub struct ValueMut<'a> {
    data: &'a mut Data,
    pub(crate) id: Id,
}

macro_rules! set_float {
    ($name:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Set the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use anyhow::Context;
        /// use nondestructive::toml;
        ///
        /// let mut doc = toml::from_slice("10")?;
        ///
        #[doc = concat!("let value = doc.as_mut().", stringify!($name), "(", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"", stringify!($lit), "\");")]
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = ryu::Buffer::new();
            let string = self.data.insert_str(buffer.format(value));
            self.data.replace(self.id, Raw::Number(raw::Number::new(string, crate::serde_hint::$hint)));
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
        /// use nondestructive::toml;
        ///
        /// let mut doc = toml::from_slice("  key = 10")?;
        ///
        #[doc = concat!("let value = doc.as_mut().get_mut(\"key\").context(\"missing key\")?.", stringify!($name), "(", stringify!($lit), ");")]
        #[doc = concat!("assert_eq!(doc.to_string(), \"  ", stringify!($lit), "\");")]
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = itoa::Buffer::new();
            let string = self.data.insert_str(buffer.format(value));
            self.data.replace(self.id, Raw::Number(raw::Number::new(string, crate::serde_hint::$hint)));
        }
    };
}

impl<'a> ValueMut<'a> {
    /// Construct a new mutable value.
    #[inline]
    pub(crate) fn new(data: &'a mut Data, id: Id) -> Self {
        Self { data, id }
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
