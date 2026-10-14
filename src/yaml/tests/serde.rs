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
