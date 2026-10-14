use core::fmt;

use bstr::{BStr, ByteSlice};

use crate::yaml::data::Data;
use crate::yaml::raw;
use crate::yaml::serde_hint::{self, RawNumberHint};

const INF: [&[u8]; 3] = [b".inf", b".Inf", b".INF"];
const NAN: [&[u8]; 3] = [b".nan", b".NaN", b".NAN"];

fn strip_sign(string: &[u8]) -> (bool, &[u8]) {
    match string {
        [b'-', rest @ ..] => (true, rest),
        [b'+', rest @ ..] => (false, rest),
        _ => (false, string),
    }
}

fn radix(string: &[u8]) -> Option<(&[u8], u32)> {
    match string {
        [b'0', b'x', rest @ ..] => Some((rest, 16)),
        [b'0', b'o', rest @ ..] => Some((rest, 8)),
        _ => None,
    }
}

fn digits(string: &[u8]) -> usize {
    string.iter().take_while(|b| b.is_ascii_digit()).count()
}

/// Resolve a plain scalar as a YAML 1.2 core schema number, returning the hint
/// to store it with.
///
/// Like libyaml (through `serde_yaml`), integers with a redundant leading zero
/// such as `007` are strings.
pub(crate) fn resolve(string: &[u8]) -> Option<RawNumberHint> {
    if let Some((rest, radix)) = radix(string) {
        let valid = !rest.is_empty() && rest.iter().all(|&b| char::from(b).is_digit(radix));
        return valid.then_some(serde_hint::U64);
    }

    let (negative, unsigned) = strip_sign(string);

    if INF.contains(&unsigned) || (unsigned.len() == string.len() && NAN.contains(&unsigned)) {
        return Some(serde_hint::F64);
    }

    let int = digits(unsigned);

    if int == unsigned.len() {
        if int == 0 || (int > 1 && unsigned[0] == b'0') {
            return None;
        }

        return Some(if negative {
            serde_hint::I64
        } else {
            serde_hint::U64
        });
    }

    let mut rest = &unsigned[int..];
    let mut mantissa = int;

    if let [b'.', tail @ ..] = rest {
        let fraction = digits(tail);
        mantissa += fraction;
        rest = &tail[fraction..];
    }

    if mantissa == 0 {
        return None;
    }

    if let [b'e' | b'E', tail @ ..] = rest {
        let (_, tail) = strip_sign(tail);
        let exponent = digits(tail);

        if exponent == 0 {
            return None;
        }

        rest = &tail[exponent..];
    }

    rest.is_empty().then_some(serde_hint::F64)
}

/// A number type which can be parsed from the text of a YAML number.
pub(crate) trait FromYaml: Sized {
    fn from_yaml(string: &[u8]) -> Option<Self>;
}

/// Parse a number written in any form [`resolve`] accepts.
pub(crate) fn parse<T>(string: &[u8]) -> Option<T>
where
    T: FromYaml,
{
    T::from_yaml(string)
}

macro_rules! integer {
    ($($ty:ty),*) => {
        $(
            impl FromYaml for $ty {
                fn from_yaml(string: &[u8]) -> Option<Self> {
                    let (digits, radix) = match radix(string) {
                        Some(radix) => radix,
                        None => (string.strip_prefix(b"+").unwrap_or(string), 10),
                    };

                    // `from_str_radix` would accept a second sign.
                    if matches!(digits, [b'+' | b'-', ..]) && (radix != 10 || digits.len() < string.len()) {
                        return None;
                    }

                    <$ty>::from_str_radix(core::str::from_utf8(digits).ok()?, radix).ok()
                }
            }
        )*
    };
}

integer!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

macro_rules! float {
    ($($ty:ident),*) => {
        $(
            impl FromYaml for $ty {
                #[allow(clippy::cast_precision_loss)]
                fn from_yaml(string: &[u8]) -> Option<Self> {
                    if radix(string).is_some() {
                        return u128::from_yaml(string).map(|value| value as $ty);
                    }

                    let (negative, unsigned) = strip_sign(string);

                    if INF.contains(&unsigned) {
                        return Some(if negative { $ty::NEG_INFINITY } else { $ty::INFINITY });
                    }

                    if NAN.contains(&unsigned) {
                        return (unsigned.len() == string.len()).then_some($ty::NAN);
                    }

                    if matches!(unsigned, [b'+' | b'-', ..]) {
                        return None;
                    }

                    let string = string.strip_prefix(b"+").unwrap_or(string);
                    lexical_core::parse(string).ok()
                }
            }
        )*
    };
}

float!(f32, f64);

macro_rules! as_number {
    ($name:ident, $ty:ty, $doc:literal, $lit:literal) => {
        #[doc = concat!("Try and get the value as a ", $doc, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use anyhow::Context;
        /// use nondestructive::yaml;
        ///
        #[doc = concat!("let doc = yaml::from_slice(\"", stringify!($lit), "\")?;")]
        /// let value = doc.as_ref().into_number().context("expected a number")?;
        #[doc = concat!("let value = value.", stringify!($name), "();")]
        #[doc = concat!("assert_eq!(value, Some(", stringify!($lit), "));")]
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        #[must_use]
        pub fn $name(&self) -> Option<$ty> {
            parse(self.data.str(self.raw.string))
        }
    };
}

/// A YAML number.
///
/// The value of the number can be accessed through the various `as_*` methods.
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use bstr::ByteSlice;
/// use nondestructive::yaml;
///
/// let a = yaml::from_slice("42")?;
/// let a = a.as_ref();
///
/// assert_eq!(a.as_u32(), Some(42));
/// # Ok::<_, anyhow::Error>(())
/// ```
///
/// [`Number`] is [`Copy`], since it only borrows from the document:
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let doc = yaml::from_slice("42")?;
/// let a = doc.as_ref().as_number().context("expected a number")?;
/// let b = a;
/// assert_eq!(a.as_raw(), b.as_raw());
/// # Ok::<_, anyhow::Error>(())
/// ```
#[derive(Clone, Copy)]
pub struct Number<'a> {
    data: &'a Data,
    raw: &'a raw::Number,
}

impl<'a> Number<'a> {
    pub(super) fn new(data: &'a Data, raw: &'a raw::Number) -> Self {
        Self { data, raw }
    }

    /// Get the raw content of the number.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_slice("3.1415")?;
    /// let doc = doc.as_ref();
    /// let value = doc.as_number().context("expected a number")?;
    /// let value = value.as_raw();
    /// assert_eq!(value, "3.1415");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn as_raw(&self) -> &BStr {
        self.data.str(self.raw.string)
    }

    as_number!(as_f32, f32, "32-bit float", 10.42);
    as_number!(as_f64, f64, "64-bit float", 10.42);
    as_number!(as_u8, u8, "8-bit unsigned integer", 42);
    as_number!(as_i8, i8, "8-bit signed integer", -42);
    as_number!(as_u16, u16, "16-bit unsigned integer", 42);
    as_number!(as_i16, i16, "16-bit signed integer", -42);
    as_number!(as_u32, u32, "16-bit unsigned integer", 42);
    as_number!(as_i32, i32, "32-bit signed integer", -42);
    as_number!(as_u64, u64, "16-bit unsigned integer", 42);
    as_number!(as_i64, i64, "64-bit signed integer", -42);
    as_number!(as_u128, u128, "16-bit unsigned integer", 42);
    as_number!(as_i128, i128, "128-bit signed integer", -42);
}

impl fmt::Debug for Number<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(string) = self.as_raw().to_str() {
            f.write_str(string)
        } else {
            f.write_str("NaN")
        }
    }
}
