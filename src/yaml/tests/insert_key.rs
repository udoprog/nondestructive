use std::format;
use std::string::{String, ToString};

use anyhow::{bail, Context, Result};

use crate::yaml;

use super::set_string::{BARE, QUOTED};

/// Keys from the original report, inserted into `a: 1`.
const REPORTED: &[(&str, &str)] = &[
    ("a: b", "'a: b'"),
    ("#x", "'#x'"),
    ("", "''"),
    ("true", "'true'"),
    ("1", "'1'"),
    ("[x]", "'[x]'"),
    ("? x", "'? x'"),
    ("?", "'?'"),
];

/// Insert `key` into each kind of mapping and check that both parsers read
/// it back as a string key holding the inserted value.
fn round_trip(key: &str) -> Result<String> {
    let mut outputs = std::vec::Vec::new();

    for input in ["a: 1", "{a: 1}", "{}", "a:\n  b: 1"] {
        let mut doc = yaml::from_slice(input)?;
        let mut mapping = doc.as_mut().into_mapping_mut().context("mapping")?;
        mapping.insert_u32(key, 2);
        outputs.push(doc.to_string());
    }

    for text in &outputs {
        let value = serde_yaml::from_str::<serde_yaml::Value>(text)
            .with_context(|| format!("serde_yaml failed on {text:?}"))?;
        let serde_yaml::Value::Mapping(mapping) = value else {
            bail!("{text:?}: expected mapping, got {value:?}");
        };
        assert_eq!(
            mapping.get(key).and_then(serde_yaml::Value::as_u64),
            Some(2),
            "serde_yaml reading {key:?} from {text:?}"
        );

        let doc = yaml::from_slice(text).with_context(|| format!("failed to parse {text:?}"))?;
        let mapping = doc.as_ref().as_mapping().context("mapping")?;
        assert_eq!(
            mapping.get(key).and_then(|v| v.as_u32()),
            Some(2),
            "nondestructive reading {key:?} from {text:?}"
        );
    }

    let text = &outputs[0];
    Ok(text
        .strip_prefix("a: 1\n")
        .and_then(|rest| rest.strip_suffix(": 2"))
        .with_context(|| format!("unexpected output {text:?}"))?
        .to_string())
}

#[test]
fn reported() -> Result<()> {
    for &(key, expected) in REPORTED {
        assert_eq!(round_trip(key)?, expected, "output for {key:?}");
    }

    Ok(())
}

#[test]
fn quoted() -> Result<()> {
    for &(key, expected) in QUOTED {
        assert_eq!(round_trip(key)?, expected, "output for {key:?}");
    }

    Ok(())
}

#[test]
fn bare() -> Result<()> {
    for &key in BARE {
        assert_eq!(round_trip(key)?, key, "output for {key:?}");
    }

    Ok(())
}

#[test]
fn replace_quoted_key() -> Result<()> {
    let mut doc = yaml::from_slice("'a: b': 1\n'true': 2")?;
    let mut mapping = doc.as_mut().into_mapping_mut().context("mapping")?;
    mapping.insert_u32("a: b", 3);
    mapping.insert_u32("true", 4);
    assert_eq!(doc.to_string(), "'a: b': 3\n'true': 4");
    Ok(())
}
