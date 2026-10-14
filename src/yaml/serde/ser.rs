use bstr::ByteSlice;

use serde::ser::{Error, SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

use crate::yaml::raw;
use crate::yaml::serde::number::Number;
use crate::yaml::{Mapping, Sequence, Value};

impl Serialize for Value<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.data.raw(self.id) {
            raw::Raw::Null(..) => serializer.serialize_none(),
            raw::Raw::Boolean(bool) => serializer.serialize_bool(bool.value),
            raw::Raw::Number(raw) => match Number::parse(self, raw.hint) {
                Some(Number::F32(value)) => serializer.serialize_f32(value),
                Some(Number::F64(value)) => serializer.serialize_f64(value),
                Some(Number::U8(value)) => serializer.serialize_u8(value),
                Some(Number::U16(value)) => serializer.serialize_u16(value),
                Some(Number::U32(value)) => serializer.serialize_u32(value),
                Some(Number::U64(value)) => serializer.serialize_u64(value),
                Some(Number::U128(value)) => serializer.serialize_u128(value),
                Some(Number::I8(value)) => serializer.serialize_i8(value),
                Some(Number::I16(value)) => serializer.serialize_i16(value),
                Some(Number::I32(value)) => serializer.serialize_i32(value),
                Some(Number::I64(value)) => serializer.serialize_i64(value),
                Some(Number::I128(value)) => serializer.serialize_i128(value),
                None => {
                    let string = self.data.str(raw.string);

                    if let Ok(string) = string.to_str() {
                        serializer.serialize_str(string)
                    } else {
                        serializer.serialize_bytes(string)
                    }
                }
            },
            raw::Raw::String(raw) => {
                let string = self.data.str(raw.id);

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

        for item in self {
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

        for (key, value) in self {
            // `key` is shadowed so that valid UTF-8 keys serialize as `str` and
            // the rest as bytes.
            match key.to_str() {
                Ok(key) => map.serialize_entry(key, &value)?,
                Err(..) => map.serialize_entry(key, &value)?,
            }
        }

        map.end()
    }
}
