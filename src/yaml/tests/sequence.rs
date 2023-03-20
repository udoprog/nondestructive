use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn sequence() -> Result<()> {
    let doc = yaml::from_slice(
        r#"
        - 1
        - 2
        - - 4
          - 5
        "#,
    )?;

    assert_eq!(
        doc.to_string(),
        r#"
        - 1
        - 2
        - - 4
          - 5
        "#,
    );

    Ok(())
}

#[test]
fn inline_sequence() -> Result<()> {
    let doc = yaml::from_slice(
        r#"
        - 1
        - 2
        - - [one, two, three]
          - 5
        "#,
    )?;

    assert_eq!(
        doc.to_string(),
        r#"
        - 1
        - 2
        - - [one, two, three]
          - 5
        "#,
    );

    Ok(())
}

#[test]
fn mutable_nested_sequence() -> Result<()> {
    let mut doc = yaml::from_slice(
        r#"
        - - 10
        "#,
    )?;

    let mut value = doc
        .root_mut()
        .into_sequence_mut()
        .context("not a sequence")?;
    let mut value = value
        .get_mut(0)
        .and_then(|v| v.into_sequence_mut())
        .expect("missing inner");
    value.push_string("nice string");

    assert_eq!(
        doc.to_string(),
        r#"
        - - 10
          - nice string
        "#
    );

    Ok(())
}
