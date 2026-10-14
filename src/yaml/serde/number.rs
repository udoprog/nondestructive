use crate::yaml::serde_hint::RawNumberHint;
use crate::yaml::Value;

/// A number parsed from its raw text, guided by the parser's hint.
#[derive(Debug, Clone, Copy)]
pub(super) enum Number {
    F32(f32),
    F64(f64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
}

impl Number {
    /// Parse `value` at its hinted width, falling back to wider types when it
    /// does not fit. Returns `None` when the text is not a number at all.
    pub(super) fn parse(value: &Value<'_>, hint: RawNumberHint) -> Option<Self> {
        let hinted = match hint {
            RawNumberHint::Float32 => value.as_f32().map(Self::F32),
            RawNumberHint::Float64 => None,
            RawNumberHint::Unsigned8 => value.as_u8().map(Self::U8),
            RawNumberHint::Unsigned16 => value.as_u16().map(Self::U16),
            RawNumberHint::Unsigned32 => value.as_u32().map(Self::U32),
            RawNumberHint::Unsigned64 => value.as_u64().map(Self::U64),
            RawNumberHint::Unsigned128 => value.as_u128().map(Self::U128),
            RawNumberHint::Signed8 => value.as_i8().map(Self::I8),
            RawNumberHint::Signed16 => value.as_i16().map(Self::I16),
            RawNumberHint::Signed32 => value.as_i32().map(Self::I32),
            RawNumberHint::Signed64 => value.as_i64().map(Self::I64),
            RawNumberHint::Signed128 => value.as_i128().map(Self::I128),
        };

        if hinted.is_some() {
            return hinted;
        }

        let wider = match hint {
            RawNumberHint::Unsigned8
            | RawNumberHint::Unsigned16
            | RawNumberHint::Unsigned32
            | RawNumberHint::Unsigned64 => value.as_u128().map(Self::U128),
            RawNumberHint::Signed8
            | RawNumberHint::Signed16
            | RawNumberHint::Signed32
            | RawNumberHint::Signed64 => value.as_i128().map(Self::I128),
            _ => None,
        };

        wider.or_else(|| value.as_f64().map(Self::F64))
    }
}
