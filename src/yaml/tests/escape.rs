use alloc::string::ToString;

use anyhow::Result;

use crate::yaml;

macro_rules! test {
    ($from:expr, $to:expr) => {
        let doc = yaml::from_slice($from)?;
        assert_eq!(doc.as_ref().as_str(), Some($to));
    };
}

#[test]
fn double_quotes() -> Result<()> {
    // Single-byte escapes
    test!("\"a \\0 b\"", "a \x00 b");
    test!("\"a \\a b\"", "a \x07 b");
    test!("\"a \\b b\"", "a \x08 b");
    test!("\"a \\t b\"", "a \x09 b");
    test!("\"a \\n b\"", "a \n b");
    test!("\"a \\v b\"", "a \x0b b");
    test!("\"a \\f b\"", "a \x0c b");
    test!("\"a \\r b\"", "a \r b");
    test!("\"a \\e b\"", "a \x1b b");
    test!("\"a \\\\ b\"", "a \\ b");
    test!("\"a \\\" b\"", "a \" b");
    test!("\"a \\  b\"", "a   b");
    test!("\"a \\/ b\"", "a / b");

    // Multi-byte Unicode escapes
    test!("\"a \\N b\"", "a \u{0085} b");
    test!("\"a \\_ b\"", "a \u{00A0} b");
    test!("\"a \\L b\"", "a \u{2028} b");
    test!("\"a \\P b\"", "a \u{2029} b");

    // Variable-length hex/unicode escapes
    test!("\"a \\x77 b\"", "a \x77 b");
    test!("\"a \\u79c1 b\"", "a 私 b");
    test!("\"a \\U00010437 b\"", "a \u{10437} b");
    Ok(())
}

#[test]
fn backslash_roundtrip() -> Result<()> {
    use crate::yaml::StringKind;
    use anyhow::Context;

    let mut doc = yaml::from_slice("key: value")?;

    if let Some(mut v) = doc
        .as_mut()
        .into_mapping_mut()
        .and_then(|m| m.get_into_mut("key"))
    {
        v.set_string_with("a\\b", StringKind::Double);
    }

    assert_eq!(doc.to_string(), "key: \"a\\\\b\"");

    let reparsed = yaml::from_slice(doc.to_string().as_bytes())?;
    let root = reparsed.as_ref().as_mapping().context("missing mapping")?;
    assert_eq!(root.get("key").and_then(|v| v.as_str()), Some("a\\b"));
    Ok(())
}

#[test]
fn single_quotes() -> Result<()> {
    test!("'It is a party!'", "It is a party!");
    test!("'It''s a party!'", "It's a party!");
    Ok(())
}
