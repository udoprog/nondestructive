mod actions;
mod escape;
mod multiline;

use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn make_mapping() -> Result<()> {
    let mut doc = yaml::from_bytes(
        r#"
    first: second
    "#,
    )?;
    let mut mapping = doc
        .root_mut()
        .into_mapping_mut()
        .and_then(|m| Some(m.get_into_mut("first")?.make_mapping()))
        .context("missing first")?;
    mapping.insert_u32("second", 2);
    mapping.insert_u32("third", 3);

    assert_eq!(
        doc.to_string(),
        r#"
    first:
      second: 2
      third: 3
    "#
    );
    Ok(())
}

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
