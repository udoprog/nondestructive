use alloc::string::ToString;
use alloc::vec::Vec;

use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn plain_multiline_scalar() -> Result<()> {
    let input = "top:\n  key1: hello world\n    continuation line\n  key2: after\n";
    let doc = yaml::from_slice(input)?;

    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    let inner = root
        .get("top")
        .and_then(|v| v.as_mapping())
        .context("missing inner mapping")?;

    assert_eq!(
        inner.get("key1").and_then(|v| v.as_str()),
        Some("hello world continuation line")
    );
    assert_eq!(inner.get("key2").and_then(|v| v.as_str()), Some("after"));

    assert_eq!(doc.to_string(), input);

    Ok(())
}

#[test]
fn string_newlines() -> Result<()> {
    let doc = yaml::from_slice(
        r"
        first: |
          foo

          bar
          baz
        second: 2
        ",
    )?;

    let root = doc.as_ref().as_mapping().context("missing root mapping")?;

    assert_eq!(
        root.get("first").and_then(|v| v.as_str()),
        Some("foo\n\nbar\nbaz\n")
    );
    assert_eq!(root.get("second").and_then(|v| v.as_u32()), Some(2));

    assert_eq!(
        doc.to_string(),
        r"
        first: |
          foo

          bar
          baz
        second: 2
        "
    );

    let doc = yaml::from_slice(
        r"
        first: | foo

          bar
          baz
        second: 2
        ",
    )?;

    let root = doc.as_ref().as_mapping().context("missing root mapping")?;

    assert_eq!(
        root.get("first").and_then(|v| v.as_str()),
        Some("foo\nbar\nbaz\n")
    );
    assert_eq!(root.get("second").and_then(|v| v.as_u32()), Some(2));

    assert_eq!(
        doc.to_string(),
        r"
        first: | foo

          bar
          baz
        second: 2
        ",
    );

    Ok(())
}

#[test]
fn string_newlines_chomped() -> Result<()> {
    let doc = yaml::from_slice(
        r"
        first: |-
          foo

          bar
          baz
        second: 2
        ",
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
        r"
        first: |+
          foo

          bar
          baz

        second: 2
        ",
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
        r"
        first: > foo

          bar
          baz
        second: 2
        ",
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
        r"
        first: > foo

          bar
          baz
        second: 2
        "
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
        r"
        first: removed
        second: 2
        "
    );

    let doc = yaml::from_slice(
        r"
        first: >
          foo

          bar
          baz
        second: 2
        ",
    )?;

    assert_eq!(
        doc.as_ref()
            .as_mapping()
            .and_then(|m| m.get("first")?.as_str()),
        Some("foo\nbar baz\n")
    );

    assert_eq!(
        doc.to_string(),
        r"
        first: >
          foo

          bar
          baz
        second: 2
        "
    );

    Ok(())
}

#[track_caller]
fn round_trip(input: &str) -> Result<yaml::Document> {
    let doc = yaml::from_slice(input)?;
    assert_eq!(doc.to_string(), input);
    let mut out = Vec::new();
    doc.write_to(&mut out)?;
    assert_eq!(out, input.as_bytes());
    Ok(doc)
}

#[test]
fn empty_block_round_trip() -> Result<()> {
    let doc = round_trip("a: |\n")?;
    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    assert_eq!(root.get("a").and_then(|v| v.as_str()), Some(""));

    let doc = round_trip("|-\n")?;
    assert_eq!(doc.as_ref().as_str(), Some(""));

    let doc = round_trip("|\n  \n")?;
    assert_eq!(doc.as_ref().as_str(), Some(""));

    let doc = round_trip("a: >\n")?;
    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    assert_eq!(root.get("a").and_then(|v| v.as_str()), Some(""));
    Ok(())
}

#[test]
fn empty_block_keeps_sibling() -> Result<()> {
    let doc = round_trip("a: |\nb: 1\n")?;
    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    assert_eq!(root.get("a").and_then(|v| v.as_str()), Some(""));
    assert_eq!(root.get("b").and_then(|v| v.as_u32()), Some(1));
    assert_eq!(root.len(), 2);

    let doc = round_trip("top:\n  a: |-\n  b: 1\nc: 2\n")?;
    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    let top = root
        .get("top")
        .and_then(|v| v.as_mapping())
        .context("missing top")?;
    assert_eq!(top.get("a").and_then(|v| v.as_str()), Some(""));
    assert_eq!(top.get("b").and_then(|v| v.as_u32()), Some(1));
    assert_eq!(root.get("c").and_then(|v| v.as_u32()), Some(2));

    let doc = round_trip("a: |\n  x\nb: 1\n")?;
    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    assert_eq!(root.get("a").and_then(|v| v.as_str()), Some("x\n"));
    assert_eq!(root.get("b").and_then(|v| v.as_u32()), Some(1));
    Ok(())
}

#[test]
fn empty_block_in_sequence() -> Result<()> {
    let doc = round_trip("- |\n- b\n")?;
    let seq = doc
        .as_ref()
        .as_sequence()
        .context("missing root sequence")?;
    assert_eq!(seq.get(0).and_then(|v| v.as_str()), Some(""));
    assert_eq!(seq.get(1).and_then(|v| v.as_str()), Some("b"));
    assert_eq!(seq.len(), 2);
    Ok(())
}

#[test]
fn block_tab_indentation() -> Result<()> {
    let doc = round_trip("a: |\n\tfoo\n\t  bar\nb: 1\n")?;
    let root = doc.as_ref().as_mapping().context("missing root mapping")?;
    assert_eq!(root.get("a").and_then(|v| v.as_str()), Some("foo\n  bar\n"));
    assert_eq!(root.get("b").and_then(|v| v.as_u32()), Some(1));
    Ok(())
}
