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
    const ALPHABET: &[u8] = b"{}[],: a1\"#\n";
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

#[test]
fn whitespace_before_separator() -> Result<()> {
    for input in [
        "[1 ,]",
        "[1\n, 2]",
        "{a: \"x\" , b: 1}",
        "[{} # c\n, b]",
        "['a'\t,\n  [b] ,{c: d}  , e ]",
        "{a: [1] # c\n , b: {} ,}",
    ] {
        let doc = yaml::from_slice(input)?;
        assert_eq!(doc.to_string(), input);

        let mut out = Vec::new();
        doc.write_to(&mut out)?;
        assert_eq!(out, input.as_bytes());
    }

    let doc = yaml::from_slice("{a: \"x\" , b: 1 }")?;
    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    assert_eq!(root.get("a").and_then(|v| v.as_str()), Some("x"));
    assert_eq!(root.get("b").and_then(|v| v.as_u32()), Some(1));
    Ok(())
}

#[test]
fn edit_after_whitespace_before_separator() -> Result<()> {
    let mut doc = yaml::from_slice("[1 , 'a' # c\n, 3]")?;
    let mut root = doc
        .as_mut()
        .into_sequence_mut()
        .context("missing root sequence")?;
    root.get_mut(0).context("missing item")?.set_u32(2);
    assert!(root.remove(2));
    assert_eq!(doc.to_string(), "[2 , 'a' # c\n]");

    let mut doc = yaml::from_slice("{a: 1 , b: 'x' ,}")?;
    let mut root = doc
        .as_mut()
        .into_mapping_mut()
        .context("missing root mapping")?;
    root.get_mut("b").context("missing b")?.set_string("y");
    assert_eq!(doc.to_string(), "{a: 1 , b: y ,}");
    Ok(())
}
