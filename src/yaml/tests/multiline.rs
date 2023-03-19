use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn string_newlines() -> Result<()> {
    let doc = yaml::from_bytes(
        r#"
        first: |
          foo

          bar
          baz
        second: 2
        "#,
    )?;

    let root = doc.root().as_mapping().context("missing root mapping")?;

    assert_eq!(
        root.get("first").and_then(|v| v.as_str()),
        Some("foo\nbar\nbaz")
    );
    assert_eq!(root.get("second").and_then(|v| v.as_u32()), Some(2));

    assert_eq!(
        doc.to_string(),
        r#"
        first: |
          foo

          bar
          baz
        second: 2
        "#
    );

    let doc = yaml::from_bytes(
        r#"
        first: | foo

          bar
          baz
        second: 2
        "#,
    )?;

    let root = doc.root().as_mapping().context("missing root mapping")?;

    assert_eq!(
        root.get("first").and_then(|v| v.as_str()),
        Some("foo\nbar\nbaz")
    );
    assert_eq!(root.get("second").and_then(|v| v.as_u32()), Some(2));

    assert_eq!(
        doc.to_string(),
        r#"
        first: | foo

          bar
          baz
        second: 2
        "#,
    );

    Ok(())
}

#[test]
fn string_spaces() -> Result<()> {
    let mut doc = yaml::from_bytes(
        r#"
        first: > foo

          bar
          baz
        second: 2
        "#,
    )?;

    assert_eq!(
        doc.root()
            .as_mapping()
            .and_then(|m| m.get("first")?.as_str()),
        Some("foo bar baz")
    );
    assert_eq!(
        doc.root()
            .as_mapping()
            .and_then(|m| m.get("second")?.as_u32()),
        Some(2)
    );

    assert_eq!(
        doc.to_string(),
        r#"
        first: > foo

          bar
          baz
        second: 2
        "#
    );

    if let Some(mut v) = doc
        .root_mut()
        .as_mapping_mut()
        .and_then(|m| m.get_into_mut("first"))
    {
        v.set_string("removed");
    }

    assert_eq!(
        doc.to_string(),
        r#"
        first: removed
        second: 2
        "#
    );

    let doc = yaml::from_bytes(
        r#"
        first: >
          foo

          bar
          baz
        second: 2
    "#,
    )?;

    assert_eq!(
        doc.root()
            .as_mapping()
            .and_then(|m| m.get("first")?.as_str()),
        Some("foo bar baz")
    );

    assert_eq!(
        doc.to_string(),
        r#"
        first: >
          foo

          bar
          baz
        second: 2
    "#
    );

    Ok(())
}