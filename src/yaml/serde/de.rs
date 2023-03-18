use bstr::{BStr, ByteSlice};
use serde::de::{self, Error as _, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserializer;

use crate::yaml::raw::RawKind;
use crate::yaml::serde::{Error, RawNumberHint};
use crate::yaml::{list, table, Document, List, Table, Value};

impl<'de, 'a: 'de> IntoDeserializer<'de, Error> for &'a Document {
    type Deserializer = Value<'de>;

    #[inline]
    fn into_deserializer(self) -> Self::Deserializer {
        self.root()
    }
}

impl<'de, 'a: 'de> IntoDeserializer<'de, Error> for Value<'a> {
    type Deserializer = Value<'de>;

    #[inline]
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

/// [`Deserializer`] implementation for [`Document`].
///
/// This allows a [`Document`] to be deserialized from any compatible type.
impl<'de> Deserializer<'de> for Value<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.data.raw(self.id).kind {
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
                let string = self.data.str(&raw.string);

                if let Ok(string) = string.to_str() {
                    visitor.visit_borrowed_str(string)
                } else {
                    visitor.visit_borrowed_bytes(string)
                }
            }
            RawKind::Table(..) => {
                visitor.visit_map(TableIter::new(Table::new(self.data, self.id).into_iter()))
            }
            RawKind::List(..) => {
                visitor.visit_seq(ListIter::new(List::new(self.data, self.id).into_iter()))
            }
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_bool() {
            Some(value) => visitor.visit_bool(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_i8() {
            Some(value) => visitor.visit_i8(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_i16() {
            Some(value) => visitor.visit_i16(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_i32() {
            Some(value) => visitor.visit_i32(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_i64() {
            Some(value) => visitor.visit_i64(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_u8() {
            Some(value) => visitor.visit_u8(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_u16() {
            Some(value) => visitor.visit_u16(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_u32() {
            Some(value) => visitor.visit_u32(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_u64() {
            Some(value) => visitor.visit_u64(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_f32() {
            Some(value) => visitor.visit_f32(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_f64() {
            Some(value) => visitor.visit_f64(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_str() {
            Some(value) => visitor.visit_borrowed_str(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_str() {
            Some(value) => visitor.visit_borrowed_str(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_str() {
            Some(value) => visitor.visit_borrowed_str(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_bstr() {
            Some(value) => visitor.visit_borrowed_bytes(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_bstr() {
            Some(value) => visitor.visit_borrowed_bytes(value),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.data.raw(self.id).kind {
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
        match self.as_list() {
            Some(value) => visitor.visit_seq(ListIter::new(value.into_iter())),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_list() {
            Some(value) => visitor.visit_seq(ListIter::new(value.into_iter())),
            None => self.deserialize_any(visitor),
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
        match self.as_table() {
            Some(value) => visitor.visit_map(TableIter::new(value.into_iter())),
            None => self.deserialize_any(visitor),
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

        seed.deserialize(value)
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

        Ok(Some(seed.deserialize(value)?))
    }
}
