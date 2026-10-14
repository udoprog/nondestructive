use bstr::{BStr, ByteSlice};
use serde::de::{
    self, EnumAccess, Error as _, IntoDeserializer, MapAccess, SeqAccess, Unexpected,
    VariantAccess, Visitor,
};
use serde::Deserializer;

use crate::yaml::raw;
use crate::yaml::serde::number::Number;
use crate::yaml::serde::Error;
use crate::yaml::{mapping, sequence, Document, Mapping, Sequence, Value};

impl<'de, 'a: 'de> IntoDeserializer<'de, Error> for &'a Document {
    type Deserializer = Value<'de>;

    #[inline]
    fn into_deserializer(self) -> Self::Deserializer {
        self.as_ref()
    }
}

impl<'de, 'a: 'de> IntoDeserializer<'de, Error> for Value<'a> {
    type Deserializer = Value<'de>;

    #[inline]
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

/// [`Deserializer`] implementation for [`Value`].
///
/// This allows a [`Value`] to be deserialized from any compatible type.
impl<'de> Deserializer<'de> for Value<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.data.raw(self.id) {
            raw::Raw::Null(..) => visitor.visit_none(),
            raw::Raw::Boolean(bool) => visitor.visit_bool(bool.value),
            raw::Raw::Number(raw) => match Number::parse(&self, raw.hint) {
                Some(Number::F32(value)) => visitor.visit_f32(value),
                Some(Number::F64(value)) => visitor.visit_f64(value),
                Some(Number::U8(value)) => visitor.visit_u8(value),
                Some(Number::U16(value)) => visitor.visit_u16(value),
                Some(Number::U32(value)) => visitor.visit_u32(value),
                Some(Number::U64(value)) => visitor.visit_u64(value),
                Some(Number::U128(value)) => visitor.visit_u128(value),
                Some(Number::I8(value)) => visitor.visit_i8(value),
                Some(Number::I16(value)) => visitor.visit_i16(value),
                Some(Number::I32(value)) => visitor.visit_i32(value),
                Some(Number::I64(value)) => visitor.visit_i64(value),
                Some(Number::I128(value)) => visitor.visit_i128(value),
                None => {
                    let string = self.data.str(raw.string);

                    if let Ok(string) = string.to_str() {
                        visitor.visit_borrowed_str(string)
                    } else {
                        visitor.visit_borrowed_bytes(string)
                    }
                }
            },
            raw::Raw::String(raw) => {
                let string = self.data.str(raw.id);

                if let Ok(string) = string.to_str() {
                    visitor.visit_borrowed_str(string)
                } else {
                    visitor.visit_borrowed_bytes(string)
                }
            }
            raw::Raw::Mapping(..) => visitor.visit_map(MappingIter::new(
                Mapping::new(self.data, self.id).into_iter(),
            )),
            raw::Raw::Sequence(..) => visitor.visit_seq(SequenceIter::new(
                Sequence::new(self.data, self.id).into_iter(),
            )),
            _ => Err(Self::Error::custom("cannot deserialize items")),
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
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_i128() {
            Some(value) => visitor.visit_i128(value),
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
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_u128() {
            Some(value) => visitor.visit_u128(value),
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
        match self.data.raw(self.id) {
            raw::Raw::Null(..) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.data.raw(self.id) {
            raw::Raw::Null(..) => visitor.visit_unit(),
            _ => self.deserialize_any(visitor),
        }
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
        self.deserialize_unit(visitor)
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
        visitor.visit_newtype_struct(self)
    }

    #[inline]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_sequence() {
            Some(value) => visitor.visit_seq(SequenceIter::new(value.into_iter())),
            None => self.deserialize_any(visitor),
        }
    }

    #[inline]
    fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.as_sequence() {
            Some(value) => visitor.visit_seq(SequenceIter::new(value.into_iter())),
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
        match self.as_mapping() {
            Some(value) => visitor.visit_map(MappingIter::new(value.into_iter())),
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
        match self.data.raw(self.id) {
            raw::Raw::String(raw) => visitor.visit_enum(UnitVariant {
                string: self.data.str(raw.id),
            }),
            raw::Raw::Mapping(..) => {
                let mut iter = Mapping::new(self.data, self.id).into_iter();

                match (iter.next(), iter.next()) {
                    (Some((variant, value)), None) => visitor.visit_enum(VariantValue {
                        variant: KeyDeserializer::new(variant),
                        value,
                    }),
                    _ => Err(Error::invalid_length(
                        Mapping::new(self.data, self.id).len(),
                        &"a mapping with a single entry",
                    )),
                }
            }
            _ => self.deserialize_any(visitor),
        }
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

/// Deserializer for mapping keys, which are stored as text and only parsed
/// into numbers or booleans when the visitor asks for one.
struct KeyDeserializer<'de> {
    string: &'de BStr,
}

impl<'de> KeyDeserializer<'de> {
    #[inline]
    fn new(string: &'de BStr) -> Self {
        Self { string }
    }

    #[inline]
    fn parse<T>(&self) -> Option<T>
    where
        T: crate::yaml::number::FromYaml,
    {
        crate::yaml::number::parse(self.string)
    }
}

macro_rules! deserialize_key_number {
    ($($method:ident, $visit:ident, $ty:ty;)*) => {
        $(
            #[inline]
            fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                match self.parse::<$ty>() {
                    Some(value) => visitor.$visit(value),
                    None => self.deserialize_any(visitor),
                }
            }
        )*
    };
}

impl<'de> Deserializer<'de> for KeyDeserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Ok(string) = self.string.to_str() {
            visitor.visit_borrowed_str(string)
        } else {
            visitor.visit_borrowed_bytes(self.string)
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if matches!(self.string.as_bytes(), b"true" | b"True" | b"TRUE") {
            visitor.visit_bool(true)
        } else if matches!(self.string.as_bytes(), b"false" | b"False" | b"FALSE") {
            visitor.visit_bool(false)
        } else {
            self.deserialize_any(visitor)
        }
    }

    deserialize_key_number! {
        deserialize_i8, visit_i8, i8;
        deserialize_i16, visit_i16, i16;
        deserialize_i32, visit_i32, i32;
        deserialize_i64, visit_i64, i64;
        deserialize_i128, visit_i128, i128;
        deserialize_u8, visit_u8, u8;
        deserialize_u16, visit_u16, u16;
        deserialize_u32, visit_u32, u32;
        deserialize_u64, visit_u64, u64;
        deserialize_u128, visit_u128, u128;
        deserialize_f32, visit_f32, f32;
        deserialize_f64, visit_f64, f64;
    }

    #[inline]
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.string)
    }

    #[inline]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.string)
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
        visitor.visit_newtype_struct(self)
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
        visitor.visit_enum(UnitVariant {
            string: self.string,
        })
    }

    serde::forward_to_deserialize_any! {
        char str string option unit unit_struct seq tuple tuple_struct map
        struct identifier ignored_any
    }
}

/// Enum given as a bare scalar, which can only name a unit variant.
struct UnitVariant<'de> {
    string: &'de BStr,
}

impl<'de> EnumAccess<'de> for UnitVariant<'de> {
    type Error = Error;
    type Variant = Self;

    #[inline]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(KeyDeserializer::new(self.string))?;
        Ok((variant, self))
    }
}

impl<'de> VariantAccess<'de> for UnitVariant<'de> {
    type Error = Error;

    #[inline]
    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn newtype_variant_seed<T>(self, _: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        Err(Error::invalid_type(
            Unexpected::UnitVariant,
            &"newtype variant",
        ))
    }

    #[inline]
    fn tuple_variant<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::invalid_type(
            Unexpected::UnitVariant,
            &"tuple variant",
        ))
    }

    #[inline]
    fn struct_variant<V>(self, _: &'static [&'static str], _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::invalid_type(
            Unexpected::UnitVariant,
            &"struct variant",
        ))
    }
}

/// Enum given as a single-entry mapping from variant name to its content.
struct VariantValue<'de> {
    variant: KeyDeserializer<'de>,
    value: Value<'de>,
}

impl<'de> EnumAccess<'de> for VariantValue<'de> {
    type Error = Error;
    type Variant = Value<'de>;

    #[inline]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.variant)?;
        Ok((variant, self.value))
    }
}

impl<'de> VariantAccess<'de> for Value<'de> {
    type Error = Error;

    #[inline]
    fn unit_variant(self) -> Result<(), Self::Error> {
        de::Deserialize::deserialize(self)
    }

    #[inline]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    #[inline]
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    #[inline]
    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_struct("", fields, visitor)
    }
}

struct MappingIter<'de> {
    iter: mapping::Iter<'de>,
    value: Option<Value<'de>>,
}

impl<'de> MappingIter<'de> {
    #[inline]
    fn new(iter: mapping::Iter<'de>) -> Self {
        Self { iter, value: None }
    }
}

impl<'de> MapAccess<'de> for MappingIter<'de> {
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
        Ok(Some(seed.deserialize(KeyDeserializer::new(key))?))
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

struct SequenceIter<'a> {
    iter: sequence::Iter<'a>,
}

impl<'de> SequenceIter<'de> {
    #[inline]
    fn new(iter: sequence::Iter<'de>) -> Self {
        Self { iter }
    }
}

impl<'de> SeqAccess<'de> for SequenceIter<'de> {
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
