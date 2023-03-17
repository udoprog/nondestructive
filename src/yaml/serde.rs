use std::fmt;

use bstr::{BStr, ByteSlice};
use serde::de::{self, Error as _, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserializer, Serialize};

use crate::yaml::raw::RawKind;
use crate::yaml::{list, table};
use crate::yaml::{Document, List, Table, Value};

/// A number hint associated with a deserialized number.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RawNumberHint {
    /// A 32-bit float.
    Float32,
    /// A 64-bit float.
    Float64,
    /// An unsigned 8-bit number.
    Unsigned8,
    /// An unsigned 16-bit number.
    Unsigned16,
    /// An unsigned 32-bit number.
    Unsigned32,
    /// An unsigned 64-bit number.
    Unsigned64,
    /// An unsigned 128-bit number.
    Unsigned128,
    /// A signed 8-bit number.
    Signed8,
    /// A signed 16-bit number.
    Signed16,
    /// A signed 32-bit number.
    Signed32,
    /// A signed 64-bit number.
    Signed64,
    /// A signed 128-bit number.
    Signed128,
}

pub(crate) const FLOAT_32: RawNumberHint = RawNumberHint::Float32;
pub(crate) const FLOAT_64: RawNumberHint = RawNumberHint::Float64;
pub(crate) const UNSIGNED_8: RawNumberHint = RawNumberHint::Unsigned8;
pub(crate) const UNSIGNED_16: RawNumberHint = RawNumberHint::Unsigned16;
pub(crate) const UNSIGNED_32: RawNumberHint = RawNumberHint::Unsigned32;
pub(crate) const UNSIGNED_64: RawNumberHint = RawNumberHint::Unsigned64;
pub(crate) const UNSIGNED_128: RawNumberHint = RawNumberHint::Unsigned128;
pub(crate) const SIGNED_8: RawNumberHint = RawNumberHint::Signed8;
pub(crate) const SIGNED_16: RawNumberHint = RawNumberHint::Signed16;
pub(crate) const SIGNED_32: RawNumberHint = RawNumberHint::Signed32;
pub(crate) const SIGNED_64: RawNumberHint = RawNumberHint::Signed64;
pub(crate) const SIGNED_128: RawNumberHint = RawNumberHint::Signed128;

impl Serialize for Document {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.root().serialize(serializer)
    }
}

impl Serialize for Value<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.raw.kind {
            RawKind::Null(..) => serializer.serialize_none(),
            RawKind::Number(raw) => match raw.hint {
                RawNumberHint::Float32 => match self.as_f32() {
                    Some(value) => serializer.serialize_f32(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Float64 => match self.as_f64() {
                    Some(value) => serializer.serialize_f64(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Unsigned8 => match self.as_u8() {
                    Some(value) => serializer.serialize_u8(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Unsigned16 => match self.as_u16() {
                    Some(value) => serializer.serialize_u16(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Unsigned32 => match self.as_u32() {
                    Some(value) => serializer.serialize_u32(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Unsigned64 => match self.as_u64() {
                    Some(value) => serializer.serialize_u64(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Unsigned128 => match self.as_u128() {
                    Some(value) => serializer.serialize_u128(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Signed8 => match self.as_i8() {
                    Some(value) => serializer.serialize_i8(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Signed16 => match self.as_i16() {
                    Some(value) => serializer.serialize_i16(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Signed32 => match self.as_i32() {
                    Some(value) => serializer.serialize_i32(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Signed64 => match self.as_i64() {
                    Some(value) => serializer.serialize_i64(value),
                    None => serializer.serialize_none(),
                },
                RawNumberHint::Signed128 => match self.as_i128() {
                    Some(value) => serializer.serialize_i128(value),
                    None => serializer.serialize_none(),
                },
            },
            RawKind::String(raw) => {
                let string = self.strings.get(&raw.string);

                if let Ok(string) = string.to_str() {
                    serializer.serialize_str(string)
                } else {
                    serializer.serialize_bytes(string)
                }
            }
            RawKind::Table(raw) => Table::new(&self.strings, raw).serialize(serializer),
            RawKind::List(raw) => List::new(&self.strings, raw).serialize(serializer),
        }
    }
}

impl Serialize for List<'_> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;

        for item in self.iter() {
            seq.serialize_element(&item)?;
        }

        seq.end()
    }
}

impl Serialize for Table<'_> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;

        for (key, value) in self.iter() {
            if let Ok(key) = key.to_str() {
                map.serialize_entry(key, &value)?;
            } else {
                map.serialize_entry(key, &value)?;
            }
        }

        map.end()
    }
}

/// A error raised during deserialization.
#[derive(Debug)]
pub struct Error {
    custom: anyhow::Error,
}

impl de::Error for Error {
    #[inline]
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self {
            custom: anyhow::Error::msg(msg.to_string()),
        }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.custom.fmt(f)
    }
}

impl std::error::Error for Error {}

impl<'de> IntoDeserializer<'de, Error> for &'de Document {
    type Deserializer = ValueDeserializer<'de>;

    #[inline]
    fn into_deserializer(self) -> Self::Deserializer {
        ValueDeserializer::new(self.root())
    }
}

pub struct ValueDeserializer<'de> {
    value: Value<'de>,
}

impl<'de> ValueDeserializer<'de> {
    pub(crate) fn new(value: Value<'de>) -> Self {
        Self { value }
    }
}

/// [`Deserializer`] implementation for [`Document`].
///
/// This allows a [`Document`] to be deserialized from any compatible type.
impl<'de> Deserializer<'de> for ValueDeserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.value.raw.kind {
            RawKind::Null(..) => visitor.visit_none(),
            RawKind::Number(raw) => match raw.hint {
                RawNumberHint::Float32 => self.deserialize_f32(visitor),
                RawNumberHint::Float64 => self.deserialize_f64(visitor),
                RawNumberHint::Unsigned8 => self.deserialize_u8(visitor),
                RawNumberHint::Unsigned16 => self.deserialize_u16(visitor),
                RawNumberHint::Unsigned32 => self.deserialize_u32(visitor),
                RawNumberHint::Unsigned64 => self.deserialize_u64(visitor),
                RawNumberHint::Unsigned128 => self.deserialize_u128(visitor),
                RawNumberHint::Signed8 => self.deserialize_i8(visitor),
                RawNumberHint::Signed16 => self.deserialize_i16(visitor),
                RawNumberHint::Signed32 => self.deserialize_i32(visitor),
                RawNumberHint::Signed64 => self.deserialize_i64(visitor),
                RawNumberHint::Signed128 => self.deserialize_i128(visitor),
            },
            RawKind::String(raw) => {
                let string = self.value.strings.get(&raw.string);

                if let Ok(string) = string.to_str() {
                    visitor.visit_borrowed_str(string)
                } else {
                    visitor.visit_borrowed_bytes(string)
                }
            }
            RawKind::Table(raw) => visitor.visit_map(TableIter::new(
                Table::new(&self.value.strings, raw).into_iter(),
            )),
            RawKind::List(raw) => visitor.visit_seq(ListIter::new(
                List::new(&self.value.strings, raw).into_iter(),
            )),
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_bool() {
            Some(value) => visitor.visit_bool(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_i8() {
            Some(value) => visitor.visit_i8(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_i16() {
            Some(value) => visitor.visit_i16(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_i32() {
            Some(value) => visitor.visit_i32(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_i64() {
            Some(value) => visitor.visit_i64(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_u8() {
            Some(value) => visitor.visit_u8(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_u16() {
            Some(value) => visitor.visit_u16(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_u32() {
            Some(value) => visitor.visit_u32(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_u64() {
            Some(value) => visitor.visit_u64(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_f32() {
            Some(value) => visitor.visit_f32(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_f64() {
            Some(value) => visitor.visit_f64(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_str() {
            Some(value) => visitor.visit_borrowed_str(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_str() {
            Some(value) => visitor.visit_borrowed_str(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_str() {
            Some(value) => visitor.visit_borrowed_str(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_bstr() {
            Some(value) => visitor.visit_borrowed_bytes(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_bstr() {
            Some(value) => visitor.visit_borrowed_bytes(value),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.raw.kind {
            RawKind::Null(..) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_unit_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_list() {
            Some(value) => visitor.visit_seq(ListIter::new(value.into_iter())),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_list() {
            Some(value) => visitor.visit_seq(ListIter::new(value.into_iter())),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.as_table() {
            Some(value) => visitor.visit_map(TableIter::new(value.into_iter())),
            None => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct BStrDeserializer<'de> {
    string: &'de BStr,
}

impl<'de> BStrDeserializer<'de> {
    #[inline]
    fn new(string: &'de BStr) -> Self {
        Self { string }
    }
}

impl<'de> Deserializer<'de> for BStrDeserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bytes(self.string)
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_unit_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct TableIter<'de> {
    iter: table::Iter<'de>,
    value: Option<Value<'de>>,
}

impl<'de> TableIter<'de> {
    #[inline]
    fn new(iter: table::Iter<'de>) -> Self {
        Self { iter, value: None }
    }
}

impl<'de> MapAccess<'de> for TableIter<'de> {
    type Error = Error;

    #[inline]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let Some((key, value)) = self.iter.next() else {
            return Ok(None);
        };

        self.value = Some(value);
        Ok(Some(seed.deserialize(BStrDeserializer::new(key))?))
    }

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let Some(value) = self.value.take() else {
            return Err(Error::custom("missing value"));
        };

        seed.deserialize(ValueDeserializer::new(value))
    }
}

struct ListIter<'a> {
    iter: list::Iter<'a>,
}

impl<'de> ListIter<'de> {
    #[inline]
    fn new(iter: list::Iter<'de>) -> Self {
        Self { iter }
    }
}

impl<'de> SeqAccess<'de> for ListIter<'de> {
    type Error = Error;

    #[inline]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let Some(value) = self.iter.next() else {
            return Ok(None);
        };

        Ok(Some(seed.deserialize(ValueDeserializer::new(value))?))
    }
}
