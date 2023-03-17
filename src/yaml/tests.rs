use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn test_property_eol() -> Result<()> {
    let doc = yaml::parse(
        r#"
        table:
            inner: so this is as a matter of @ course, a large document
        string3: "I am a quoted string!"
        "#,
    )?;

    let root = doc.root().as_table().context("missing root table")?;

    let table = root
        .get("table")
        .and_then(|v| v.as_table())
        .context("missing inner table")?;

    let string = table.get("inner").and_then(|v| v.as_str());
    assert_eq!(
        string,
        Some("so this is as a matter of @ course, a large document")
    );
    Ok(())
}

#[test]
fn test_lists() -> Result<()> {
    let doc = yaml::parse(
        r#"
        - one
        - two
        - - three
          - four: 2
            five: 1
        - six
        "#,
    )?;

    let root = doc.root().as_list().context("missing root list")?;

    assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));

    let three = root
        .get(2)
        .and_then(|v| v.as_list())
        .context("missing three")?;

    assert_eq!(three.get(0).and_then(|v| v.as_str()), Some("three"));

    let four = three
        .get(1)
        .and_then(|v| v.as_table())
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
    let doc = yaml::parse(ACTION)?;
    assert_eq!(doc.to_string(), ACTION);
    Ok(())
}
