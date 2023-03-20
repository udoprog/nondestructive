use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn string_newlines() -> Result<()> {
    let doc = yaml::from_slice(
        r#"
        first: |
          foo

          bar
          baz
        second: 2
        "#,
    )?;

    let root = doc.as_ref().as_mapping().context("missing root mapping")?;

    assert_eq!(
        root.get("first").and_then(|v| v.as_str()),
        Some("foo\n\nbar\nbaz\n")
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

    let doc = yaml::from_slice(
        r#"
        first: | foo

          bar
          baz
        second: 2
        "#,
    )?;

    let root = doc.as_ref().as_mapping().context("missing root mapping")?;

    assert_eq!(
        root.get("first").and_then(|v| v.as_str()),
        Some("foo\nbar\nbaz\n")
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
fn string_newlines_chomped() -> Result<()> {
    let doc = yaml::from_slice(
        r#"
        first: |-
          foo

          bar
          baz
        second: 2
        "#,
    )?;

    let root = doc.as_ref().as_mapping().context("missing root mapping")?;

    assert_eq!(
        root.get("first").and_then(|v| v.as_str()),
        Some("foo\n\nbar\nbaz")
    );
    assert_eq!(root.get("second").and_then(|v| v.as_u32()), Some(2));
    Ok(())
}

#[test]
fn string_newlines_keep() -> Result<()> {
    let doc = yaml::from_slice(
        r#"
        first: |+
          foo

          bar
          baz

        second: 2
        "#,
    )?;

    let root = doc.as_ref().as_mapping().context("missing root mapping")?;

    assert_eq!(
        root.get("first").and_then(|v| v.as_str()),
        Some("foo\n\nbar\nbaz\n\n")
    );
    assert_eq!(root.get("second").and_then(|v| v.as_u32()), Some(2));
    Ok(())
}

#[test]
fn string_spaces() -> Result<()> {
    let mut doc = yaml::from_slice(
        r#"
        first: > foo

          bar
          baz
        second: 2
        "#,
    )?;

    assert_eq!(
        doc.as_ref()
            .as_mapping()
            .and_then(|m| m.get("first")?.as_str()),
        Some("foo bar baz\n")
    );
    assert_eq!(
        doc.as_ref()
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
        .as_mut()
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

    let doc = yaml::from_slice(
        r#"
        first: >
          foo

          bar
          baz
        second: 2
    "#,
    )?;

    assert_eq!(
        doc.as_ref()
            .as_mapping()
            .and_then(|m| m.get("first")?.as_str()),
        Some("foo bar baz\n")
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
