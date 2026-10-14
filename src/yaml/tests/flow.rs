use alloc::string::ToString;
use alloc::vec::Vec;

use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn colon_inside_plain_scalar() -> Result<()> {
    let doc = yaml::from_slice("{url: http://x.com, a:b: c}")?;
    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    assert_eq!(root.len(), 2);
    assert_eq!(
        root.get("url").and_then(|v| v.as_str()),
        Some("http://x.com")
    );
    assert_eq!(root.get("a:b").and_then(|v| v.as_str()), Some("c"));
    assert_eq!(doc.to_string(), "{url: http://x.com, a:b: c}");

    let doc = yaml::from_slice("[http://x.com, b]")?;
    let root = doc
        .as_ref()
        .as_sequence()
        .context("missing root sequence")?;
    assert_eq!(root.len(), 2);
    assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("http://x.com"));
    assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("b"));
    assert_eq!(doc.to_string(), "[http://x.com, b]");
    Ok(())
}

#[test]
fn quoted_key_adjacent_colon() -> Result<()> {
    let doc = yaml::from_slice(r#"{"a":1, 'b':2}"#)?;
    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    assert_eq!(root.len(), 2);
    assert_eq!(doc.to_string(), r#"{"a":1, 'b':2}"#);
    Ok(())
}

#[test]
fn number_followed_by_text_is_a_string() -> Result<()> {
    let doc = yaml::from_slice("[1 b, 1.2.3, 4]")?;
    let root = doc
        .as_ref()
        .as_sequence()
        .context("missing root sequence")?;
    assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("1 b"));
    assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("1.2.3"));
    assert_eq!(root.get(2).and_then(|v| v.as_u32()), Some(4));
    assert_eq!(doc.to_string(), "[1 b, 1.2.3, 4]");
    Ok(())
}

#[test]
fn missing_separator_is_an_error() {
    let err = yaml::from_slice("{a: 1 b: 2}").unwrap_err();
    assert_eq!(*err.kind(), yaml::ErrorKind::BadMappingTerminator);

    let err = yaml::from_slice("{a: [1] b: 2}").unwrap_err();
    assert_eq!(*err.kind(), yaml::ErrorKind::BadMappingTerminator);

    let err = yaml::from_slice("[[1] [2]]").unwrap_err();
    assert_eq!(*err.kind(), yaml::ErrorKind::BadSequenceTerminator);
}

#[test]
fn key_without_colon_is_an_error() {
    let err = yaml::from_slice("{a, b: 1}").unwrap_err();
    assert_eq!(*err.kind(), yaml::ErrorKind::BadMappingSeparator);

    let err = yaml::from_slice("{a: 1, b}").unwrap_err();
    assert_eq!(*err.kind(), yaml::ErrorKind::BadMappingSeparator);
}

/// Every flow collection over a small alphabet that parses must round-trip
/// byte-for-byte.
#[test]
fn flow_round_trip() {
    const ALPHABET: &[u8] = b"{}[],: a1\"";
    const MAX: u32 = 5;

    let mut input = Vec::new();

    for (open, close) in [(b'{', b'}'), (b'[', b']')] {
        for len in 0..=MAX {
            for mut n in 0..ALPHABET.len().pow(len) {
                input.clear();
                input.push(open);

                for _ in 0..len {
                    input.push(ALPHABET[n % ALPHABET.len()]);
                    n /= ALPHABET.len();
                }

                input.push(close);

                // Whitespace between a non-plain value and `,` has no slot
                // in the raw model and is not preserved.
                if input.windows(2).any(|w| w == b" ,") {
                    continue;
                }

                let Ok(doc) = yaml::from_slice(&input) else {
                    continue;
                };

                assert_eq!(
                    doc.to_string().as_bytes(),
                    &input[..],
                    "{:?}",
                    bstr::BStr::new(&input)
                );
            }
        }
    }
}
