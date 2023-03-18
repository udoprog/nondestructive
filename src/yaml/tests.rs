use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn test_property_eol() -> Result<()> {
    let doc = yaml::from_bytes(
        r#"
        mapping:
            inner: so this is as a matter of @ course, a large document
        string3: "I am a quoted string!"
        "#,
    )?;

    let root = doc.root().as_mapping().context("missing root mapping")?;

    let mapping = root
        .get("mapping")
        .and_then(|v| v.as_mapping())
        .context("missing inner mapping")?;

    let string = mapping.get("inner").and_then(|v| v.as_str());
    assert_eq!(
        string,
        Some("so this is as a matter of @ course, a large document")
    );
    Ok(())
}

#[test]
fn test_sequences() -> Result<()> {
    let doc = yaml::from_bytes(
        r#"
        - one
        - two
        - - three
          - four: 2
            five: 1
        - six
        "#,
    )?;

    let root = doc.root().as_sequence().context("missing root sequence")?;

    assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));

    let three = root
        .get(2)
        .and_then(|v| v.as_sequence())
        .context("missing three")?;

    assert_eq!(three.get(0).and_then(|v| v.as_str()), Some("three"));

    let four = three
        .get(1)
        .and_then(|v| v.as_mapping())
        .context("missing four")?;

    assert_eq!(four.get("four").and_then(|v| v.as_u32()), Some(2));
    assert_eq!(four.get("five").and_then(|v| v.as_u32()), Some(1));

    assert_eq!(root.get(3).and_then(|v| v.as_str()), Some("six"));
    Ok(())
}

/// Just roundtrip a fairly complex document to ensure it's correctly processed.
#[test]
fn test_actions() -> Result<()> {
    const ACTION: &str = include_str!("tests/actions.yaml");
    let doc = yaml::from_bytes(ACTION)?;
    assert_eq!(doc.to_string(), ACTION);
    Ok(())
}

#[test]
fn test_double_quoted_escapes() -> Result<()> {
    macro_rules! test {
        ($from:expr, $to:expr) => {
            let doc = yaml::from_bytes($from)?;
            assert_eq!(doc.root().as_str(), Some($to));
        };
    }

    test!("\"a \\n b\"", "a \n b");
    test!("\"a \\0 b\"", "a \x00 b");
    test!("\"a \\a b\"", "a \x07 b");
    test!("\"a \\b b\"", "a \x08 b");
    test!("\"a \\t b\"", "a \x09 b");
    test!("\"a \\v b\"", "a \x0b b");
    test!("\"a \\f b\"", "a \x0c b");
    test!("\"a \\r b\"", "a \r b");
    test!("\"a \\e b\"", "a \x1b b");
    test!("\"a \\\\ b\"", "a \" b");

    test!("\"a \\x77 b\"", "a \x77 b");
    test!("\"a \\u79c1 b\"", "a 私 b");
    Ok(())
}

#[test]
fn test_single_quoted_escapes() -> Result<()> {
    macro_rules! test {
        ($from:expr, $to:expr) => {
            let doc = yaml::from_bytes($from)?;
            assert_eq!(doc.root().as_str(), Some($to));
        };
    }

    test!("'It is a party!'", "It is a party!");
    test!("'It''s a party!'", "It's a party!");
    Ok(())
}
