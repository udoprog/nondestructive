use bstr::ByteSlice;

use serde::ser::{Error, SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

use crate::yaml::raw;
use crate::yaml::serde::RawNumberHint;
use crate::yaml::{Document, Mapping, Sequence, Value};

impl Serialize for Document {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.root().serialize(serializer)
    }
}

impl Serialize for Value<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.data.raw(self.id) {
            raw::Raw::Null(..) => serializer.serialize_none(),
            raw::Raw::Boolean(value) => serializer.serialize_bool(*value),
            raw::Raw::Number(raw) => match raw.hint {
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
            raw::Raw::String(raw) => {
                let string = self.data.str(raw.string);

                if let Ok(string) = string.to_str() {
                    serializer.serialize_str(string)
                } else {
                    serializer.serialize_bytes(string)
                }
            }
            raw::Raw::Mapping(..) => Mapping::new(self.data, self.id).serialize(serializer),
            raw::Raw::Sequence(..) => Sequence::new(self.data, self.id).serialize(serializer),
            _ => Err(S::Error::custom("cannot serialize items")),
        }
    }
}

impl Serialize for Sequence<'_> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;

        for item in self.iter() {
            seq.serialize_element(&item)?;
        }

        seq.end()
    }
}

impl Serialize for Mapping<'_> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
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
