use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn mapping() -> Result<()> {
    let doc = yaml::from_bytes(
        r#"
        one: 1
        two: 2
        three:
          four: 4
          five: 5
        "#,
    )?;

    assert_eq!(
        doc.to_string(),
        r#"
        one: 1
        two: 2
        three:
          four: 4
          five: 5
        "#,
    );

    Ok(())
}

#[test]
fn inline_mapping() -> Result<()> {
    let doc = yaml::from_bytes(
        r#"
        one: 1
        two: 2
        three:
          four: {inner: 10, inner2: 20}
          five: 5
        "#,
    )?;

    assert_eq!(
        doc.to_string(),
        r#"
        one: 1
        two: 2
        three:
          four: {inner: 10, inner2: 20}
          five: 5
        "#,
    );

    Ok(())
}

#[test]
fn inline_mapping_only() -> Result<()> {
    let doc = yaml::from_bytes(
        r#"
        {one: one, two: two, three: 3,}
        "#,
    )?;

    let root = doc.root().as_mapping().context("missing root mapping")?;
    assert_eq!(root.get("one").and_then(|v| v.as_str()), Some("one"));
    assert_eq!(root.get("two").and_then(|v| v.as_str()), Some("two"));
    assert_eq!(root.get("three").and_then(|v| v.as_u32()), Some(3));

    assert_eq!(
        doc.to_string(),
        r#"
        {one: one, two: two, three: 3,}
        "#,
    );

    Ok(())
}

#[test]
fn make_preserve_whitespace() -> Result<()> {
    let mut doc = yaml::from_bytes(
        r#"
        string
        "#,
    )?;

    let mut mapping = doc.root_mut().make_mapping();
    mapping.insert_u32("first", 1);
    mapping.insert_u32("second", 2);

    assert_eq!(
        doc.to_string(),
        r#"
        first: 1
        second: 2
        "#
    );

    Ok(())
}
