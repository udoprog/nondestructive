#![cfg(feature = "serde")]

use std::collections::{BTreeMap, HashMap};
use std::string::String;

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

#[test]
fn borrowed_str_keys() -> Result<()> {
    let doc = yaml::from_slice("a: 1\nb: 2\n")?;
    let map = HashMap::<&str, u32>::deserialize(doc.into_deserializer())?;
    assert_eq!(map, HashMap::from([("a", 1), ("b", 2)]));
    let map = HashMap::<String, u32>::deserialize(doc.into_deserializer())?;
    assert_eq!(map, HashMap::from([("a".into(), 1), ("b".into(), 2)]));
    Ok(())
}

#[test]
fn integer_keys() -> Result<()> {
    #[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
    struct Id(u128);

    let doc = yaml::from_slice("1: a\n2: b\n")?;
    let map = BTreeMap::<u32, String>::deserialize(doc.into_deserializer())?;
    assert_eq!(map, BTreeMap::from([(1, "a".into()), (2, "b".into())]));

    let doc = yaml::from_slice("-1: a\n")?;
    let map = BTreeMap::<i64, &str>::deserialize(doc.into_deserializer())?;
    assert_eq!(map, BTreeMap::from([(-1, "a")]));

    let doc = yaml::from_slice("340282366920938463463374607431768211455: a\n")?;
    let map = BTreeMap::<u128, &str>::deserialize(doc.into_deserializer())?;
    assert_eq!(map, BTreeMap::from([(u128::MAX, "a")]));
    assert!(BTreeMap::<u64, &str>::deserialize(doc.into_deserializer()).is_err());

    let doc = yaml::from_slice("7: a\n")?;
    let map = BTreeMap::<Id, &str>::deserialize(doc.into_deserializer())?;
    assert_eq!(map, BTreeMap::from([(Id(7), "a")]));

    let doc = yaml::from_slice("x: a\n")?;
    assert!(BTreeMap::<u32, &str>::deserialize(doc.into_deserializer()).is_err());
    Ok(())
}

#[test]
fn bool_keys() -> Result<()> {
    let doc = yaml::from_slice("true: a\nFalse: b\n")?;
    let map = BTreeMap::<bool, &str>::deserialize(doc.into_deserializer())?;
    assert_eq!(map, BTreeMap::from([(true, "a"), (false, "b")]));
    Ok(())
}

#[test]
fn struct_field_keys() -> Result<()> {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Fields<'a> {
        name: &'a str,
        #[serde(rename = "1")]
        one: u32,
    }

    let doc = yaml::from_slice("name: x\n1: 2\nignored: [1, 2]\n")?;
    assert_eq!(
        Fields::deserialize(doc.into_deserializer())?,
        Fields { name: "x", one: 2 }
    );
    Ok(())
}

#[test]
fn non_utf8_keys() -> Result<()> {
    let doc = yaml::from_slice(b"\xff: a\n")?;
    let map = BTreeMap::<&[u8], &str>::deserialize(doc.into_deserializer())?;
    assert_eq!(map, BTreeMap::from([(&b"\xff"[..], "a")]));
    Ok(())
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
enum Shape<'a> {
    Empty,
    Named(&'a str),
    Point(i32, i32),
    Circle { r: u32 },
}

#[test]
fn enum_variants() -> Result<()> {
    for (source, expected) in [
        ("Empty\n", Shape::Empty),
        ("Empty: ~\n", Shape::Empty),
        ("Named: x\n", Shape::Named("x")),
        ("Point: [1, -2]\n", Shape::Point(1, -2)),
        ("Point:\n- 1\n- -2\n", Shape::Point(1, -2)),
        ("Circle: {r: 2}\n", Shape::Circle { r: 2 }),
        ("Circle:\n  r: 2\n", Shape::Circle { r: 2 }),
    ] {
        let doc = yaml::from_slice(source)?;
        assert_eq!(Shape::deserialize(doc.into_deserializer())?, expected);
    }

    Ok(())
}

#[test]
fn enum_field() -> Result<()> {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Color {
        Red,
        Green,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct C<'a> {
        color: Color,
        #[serde(borrow)]
        shape: Shape<'a>,
    }

    let doc = yaml::from_slice("color: Green\nshape:\n  Circle: {r: 2}\n")?;
    assert_eq!(
        C::deserialize(doc.into_deserializer())?,
        C {
            color: Color::Green,
            shape: Shape::Circle { r: 2 }
        }
    );

    let doc = yaml::from_slice("color: Red\nshape: Empty\n")?;
    assert_eq!(
        C::deserialize(doc.into_deserializer())?,
        C {
            color: Color::Red,
            shape: Shape::Empty
        }
    );
    Ok(())
}

#[test]
fn enum_keys() -> Result<()> {
    let doc = yaml::from_slice("Empty: 1\n")?;
    let map = BTreeMap::<Shape, u32>::deserialize(doc.into_deserializer())?;
    assert_eq!(map, BTreeMap::from([(Shape::Empty, 1)]));

    let doc = yaml::from_slice("Named: 1\n")?;
    assert!(BTreeMap::<Shape, u32>::deserialize(doc.into_deserializer()).is_err());
    Ok(())
}

#[test]
fn enum_errors() -> Result<()> {
    for source in [
        "Unknown\n",
        "Named\n",
        "Empty: 1\n",
        "Named: [1]\n",
        "Point: [1]\n",
        "Circle: 2\n",
        "{}\n",
        "Empty: ~\nNamed: x\n",
        "[Empty]\n",
        "1\n",
        "~\n",
    ] {
        let doc = yaml::from_slice(source)?;
        assert!(
            Shape::deserialize(doc.into_deserializer()).is_err(),
            "{source:?}"
        );
    }

    Ok(())
}

#[test]
fn serialize_out_of_range_numbers() -> Result<()> {
    let doc = yaml::from_slice("x: 99999999999999999999\ny: -99999999999999999999\nz: 1e999\n")?;
    let out = serde_yaml::to_string(&doc.as_ref())?;
    assert_eq!(
        out,
        "x: 99999999999999999999\ny: -99999999999999999999\nz: .inf\n"
    );
    Ok(())
}

#[test]
fn serialize_integer_overflow_widens_to_float() -> Result<()> {
    let doc = yaml::from_slice("999999999999999999999999999999999999999999\n")?;
    let value = serde_yaml::to_value(doc.as_ref())?;
    let value = value.as_f64().expect("float");
    assert!((value - 1e42).abs() < 1e27);
    Ok(())
}

#[test]
fn serialize_non_numbers_as_strings() -> Result<()> {
    for (source, text) in [("x: .\n", "."), ("x: 1e\n", "1e")] {
        let doc = yaml::from_slice(source)?;
        let value = serde_yaml::to_value(doc.as_ref())?;
        assert_eq!(value["x"].as_str(), Some(text));
    }

    Ok(())
}
