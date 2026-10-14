use std::format;
use std::string::{String, ToString};

use anyhow::{bail, Context, Result};

use crate::yaml;

/// Strings that must be quoted, with the exact output expected.
pub(super) const QUOTED: &[(&str, &str)] = &[
    ("", "''"),
    ("~", "'~'"),
    ("null", "'null'"),
    ("NULL", "'NULL'"),
    ("Null", "'Null'"),
    ("true", "'true'"),
    ("True", "'True'"),
    ("FALSE", "'FALSE'"),
    ("#x", "'#x'"),
    ("&x", "'&x'"),
    ("*x", "'*x'"),
    ("!x", "'!x'"),
    ("|", "'|'"),
    (">", "'>'"),
    ("%x", "'%x'"),
    ("@x", "'@x'"),
    ("`x", "'`x'"),
    ("\"x", "'\"x'"),
    ("'x", "\"'x\""),
    ("?x", "'?x'"),
    (":x", "':x'"),
    ("- x", "'- x'"),
    ("-", "'-'"),
    ("---", "'---'"),
    ("...", "'...'"),
    ("[x]", "'[x]'"),
    ("{x}", "'{x}'"),
    ("a, b", "'a, b'"),
    ("a #b", "'a #b'"),
    ("a: b", "'a: b'"),
    ("a:", "'a:'"),
    (" lead", "' lead'"),
    ("trail ", "'trail '"),
    ("1", "'1'"),
    ("-1", "'-1'"),
    ("+1", "'+1'"),
    (".5", "'.5'"),
    ("-.5", "'-.5'"),
    ("1.65", "'1.65'"),
    ("1e3", "'1e3'"),
    ("007", "'007'"),
    ("0x1F", "'0x1F'"),
    ("+0x1F", "'+0x1F'"),
    ("-0x1F", "'-0x1F'"),
    ("0o17", "'0o17'"),
    ("0b101", "'0b101'"),
    (".inf", "'.inf'"),
    ("-.Inf", "'-.Inf'"),
    (".NaN", "'.NaN'"),
    ("1st", "'1st'"),
    ("tab\there", "\"tab\\there\""),
    ("line\nbreak", "\"line\\nbreak\""),
    ("c1\u{80}", "\"c1\\x80\""),
    ("ls\u{2028}", "\"ls\\L\""),
    ("\u{feff}bom", "\"\\uFEFFbom\""),
];

/// Strings that stay bare.
pub(super) const BARE: &[&str] = &[
    "x",
    "hello world",
    "i-am-a-string",
    "rust@1.65",
    "a:b",
    "a#b",
    "-x",
    "--release",
    "x-",
    "inf",
    "nan",
    ".x",
    "yes",
    "nulls",
    "truex",
    "I am a string with \"quotes\"",
];

fn serde_yaml_str(text: &str) -> Result<String> {
    match serde_yaml::from_str::<serde_yaml::Value>(text)
        .with_context(|| format!("serde_yaml failed on {text:?}"))?
    {
        serde_yaml::Value::String(string) => Ok(string),
        serde_yaml::Value::Mapping(mut mapping) => match mapping.remove("k") {
            Some(serde_yaml::Value::String(string)) => Ok(string),
            other => bail!("{text:?}: expected string under k, got {other:?}"),
        },
        serde_yaml::Value::Sequence(mut sequence) => match sequence.pop() {
            Some(serde_yaml::Value::String(string)) => Ok(string),
            other => bail!("{text:?}: expected string item, got {other:?}"),
        },
        other => bail!("{text:?}: expected string, got {other:?}"),
    }
}

fn own_str(text: &str) -> Result<String> {
    let doc = yaml::from_slice(text).with_context(|| format!("failed to parse {text:?}"))?;
    let value = doc.as_ref();

    let string = if let Some(mapping) = value.as_mapping() {
        mapping
            .get("k")
            .context("missing k")?
            .as_str()
            .map(str::to_string)
    } else if let Some(sequence) = value.as_sequence() {
        let last = sequence.len().checked_sub(1).context("empty sequence")?;
        sequence
            .get(last)
            .context("missing item")?
            .as_str()
            .map(str::to_string)
    } else {
        value.as_str().map(str::to_string)
    };

    string.with_context(|| format!("{text:?}: not a string"))
}

/// Write `string` into each kind of container and check that it reads back
/// unchanged through both parsers.
fn round_trip(string: &str) -> Result<String> {
    let mut outputs = std::vec::Vec::new();

    for input in ["k: v", "{k: v}", "- v", "[v]", "v"] {
        let mut doc = yaml::from_slice(input)?;
        let value = doc.as_mut();

        if let Some(mut mapping) = value.into_mapping_mut() {
            mapping
                .get_mut("k")
                .context("missing k")?
                .set_string(string);
        } else if let Some(mut sequence) = doc.as_mut().into_sequence_mut() {
            sequence
                .get_mut(0)
                .context("missing item")?
                .set_string(string);
        } else {
            doc.as_mut().set_string(string);
        }

        outputs.push(doc.to_string());
    }

    let mut doc = yaml::from_slice("k: v")?;
    let mut mapping = doc.as_mut().into_mapping_mut().context("mapping")?;
    mapping.remove("k");
    mapping.insert_str("k", string);
    outputs.push(doc.to_string());

    let mut doc = yaml::from_slice("- v")?;
    let mut sequence = doc.as_mut().into_sequence_mut().context("sequence")?;
    sequence.push_string(string);
    outputs.push(doc.to_string());

    for text in &outputs {
        assert_eq!(serde_yaml_str(text)?, string, "serde_yaml reading {text:?}");
        assert_eq!(own_str(text)?, string, "nondestructive reading {text:?}");
    }

    let text = &outputs[0];
    Ok(text
        .strip_prefix("k: ")
        .context("missing prefix")?
        .to_string())
}

#[test]
fn quoted() -> Result<()> {
    for &(string, expected) in QUOTED {
        assert_eq!(round_trip(string)?, expected, "output for {string:?}");
    }

    Ok(())
}

#[test]
fn bare() -> Result<()> {
    for &string in BARE {
        assert_eq!(round_trip(string)?, string, "output for {string:?}");
    }

    Ok(())
}
