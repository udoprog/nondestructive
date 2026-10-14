#![allow(clippy::cast_possible_truncation)]

use std::collections::BTreeMap;
use std::format;
use std::string::{String, ToString};
use std::vec::Vec;

use anyhow::{Context, Result};

use crate::yaml;

fn check(text: &str, expected: f64) -> Result<()> {
    let value: f64 = serde_yaml::from_str(text)?;

    if expected.is_nan() {
        assert!(value.is_nan(), "{text:?}");
    } else {
        assert_eq!(value, expected, "{text:?}");
    }

    Ok(())
}

const CASES: [(f64, &str); 3] = [
    (f64::NAN, ".nan"),
    (f64::INFINITY, ".inf"),
    (f64::NEG_INFINITY, "-.inf"),
];

#[test]
fn set_non_finite() -> Result<()> {
    for (value, text) in CASES {
        let mut doc = yaml::from_slice("10")?;
        doc.as_mut().set_f64(value);
        assert_eq!(doc.to_string(), text);
        check(&doc.to_string(), value)?;

        let mut doc = yaml::from_slice("10")?;
        doc.as_mut().set_f32(value as f32);
        assert_eq!(doc.to_string(), text);
        check(&doc.to_string(), value)?;
    }

    Ok(())
}

#[test]
fn insert_non_finite() -> Result<()> {
    for (value, text) in CASES {
        let mut doc = yaml::from_slice("a: 1\n")?;
        let mut m = doc.as_mut().into_mapping_mut().context("not a mapping")?;
        m.insert_f64("b", value);
        m.insert_f32("c", value as f32);
        assert_eq!(doc.to_string(), format!("a: 1\nb: {text}\nc: {text}\n"));

        let map: BTreeMap<String, f64> = serde_yaml::from_str(&doc.to_string())?;
        for key in ["b", "c"] {
            if value.is_nan() {
                assert!(map[key].is_nan());
            } else {
                assert_eq!(map[key], value);
            }
        }
    }

    Ok(())
}

#[test]
fn push_non_finite() -> Result<()> {
    for (value, text) in CASES {
        let mut doc = yaml::from_slice("- 1\n")?;
        let mut s = doc.as_mut().into_sequence_mut().context("not a sequence")?;
        s.push_f64(value);
        s.push_f32(value as f32);
        assert_eq!(doc.to_string(), format!("- 1\n- {text}\n- {text}\n"));

        let items: Vec<f64> = serde_yaml::from_str(&doc.to_string())?;
        for item in &items[1..] {
            if value.is_nan() {
                assert!(item.is_nan());
            } else {
                assert_eq!(*item, value);
            }
        }
    }

    Ok(())
}

#[test]
fn finite_unchanged() -> Result<()> {
    let mut doc = yaml::from_slice("10")?;
    doc.as_mut().set_f64(1e300);
    assert_eq!(doc.to_string(), "1e300");
    doc.as_mut().set_f64(1.0);
    assert_eq!(doc.to_string(), "1.0");
    Ok(())
}
