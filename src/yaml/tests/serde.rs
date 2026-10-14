#![cfg(feature = "serde")]

use anyhow::Result;
use serde::de::{IgnoredAny, IntoDeserializer};
use serde::Deserialize;

use crate::yaml;

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct U64 {
    x: u64,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Str<'a> {
    x: &'a str,
}

#[test]
fn u64_overflow() -> Result<()> {
    let doc = yaml::from_slice("x: 99999999999999999999\n")?;
    IgnoredAny::deserialize(doc.into_deserializer())?;
    assert!(U64::deserialize(doc.into_deserializer()).is_err());
    Ok(())
}

#[test]
fn i64_overflow() -> Result<()> {
    let doc = yaml::from_slice("x: -99999999999999999999\n")?;
    IgnoredAny::deserialize(doc.into_deserializer())?;
    assert!(U64::deserialize(doc.into_deserializer()).is_err());
    Ok(())
}

#[test]
fn integer_overflow_widens_to_float() -> Result<()> {
    let doc = yaml::from_slice("999999999999999999999999999999999999999999\n")?;
    let value = f64::deserialize(doc.into_deserializer())?;
    assert!((value - 1e42).abs() < 1e27);
    Ok(())
}

#[test]
fn non_numbers() -> Result<()> {
    for (source, text) in [("x: .\n", "."), ("x: 1e\n", "1e")] {
        let doc = yaml::from_slice(source)?;
        IgnoredAny::deserialize(doc.into_deserializer())?;
        assert!(U64::deserialize(doc.into_deserializer()).is_err());
        assert_eq!(Str::deserialize(doc.into_deserializer())?, Str { x: text });
    }

    Ok(())
}

#[test]
fn unit() -> Result<()> {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Unit {
        u: (),
    }

    for source in ["u: ~\n", "u: null\n"] {
        let doc = yaml::from_slice(source)?;
        assert_eq!(Unit::deserialize(doc.into_deserializer())?, Unit { u: () });
    }

    let doc = yaml::from_slice("u: 1\n")?;
    assert!(Unit::deserialize(doc.into_deserializer()).is_err());
    Ok(())
}

#[test]
fn unit_struct() -> Result<()> {
    #[derive(Debug, Deserialize, PartialEq)]
    struct U;

    let doc = yaml::from_slice("~\n")?;
    assert_eq!(U::deserialize(doc.into_deserializer())?, U);
    Ok(())
}

#[test]
fn newtype_struct() -> Result<()> {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Nt(u32);

    let doc = yaml::from_slice("5\n")?;
    assert_eq!(Nt::deserialize(doc.into_deserializer())?, Nt(5));
    Ok(())
}

#[test]
fn u128_and_i128() -> Result<()> {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Wide {
        x: u128,
        y: i128,
    }

    let doc = yaml::from_slice("x: 5\ny: -5\n")?;
    assert_eq!(
        Wide::deserialize(doc.into_deserializer())?,
        Wide { x: 5, y: -5 }
    );

    let doc = yaml::from_slice(
        "x: 340282366920938463463374607431768211455\ny: -170141183460469231731687303715884105728\n",
    )?;
    assert_eq!(
        Wide::deserialize(doc.into_deserializer())?,
        Wide {
            x: u128::MAX,
            y: i128::MIN
        }
    );
    Ok(())
}

#[test]
fn set_u128_deserialize_any() -> Result<()> {
    struct Any;

    impl serde::de::Visitor<'_> for Any {
        type Value = u128;

        fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            f.write_str("u128")
        }

        fn visit_u128<E>(self, value: u128) -> Result<u128, E> {
            Ok(value)
        }
    }

    let mut doc = yaml::from_slice("0\n")?;
    doc.as_mut().set_u128(u128::MAX);
    assert_eq!(u128::deserialize(doc.into_deserializer())?, u128::MAX);
    assert_eq!(
        serde::Deserializer::deserialize_any(doc.as_ref(), Any)?,
        u128::MAX
    );
    Ok(())
}
