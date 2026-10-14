use alloc::string::ToString;

use anyhow::{Context, Result};

use crate::yaml::{self, Any, Value};

fn is_null(value: Option<Value<'_>>) -> bool {
    matches!(value.map(Value::into_any), Some(Any::Null))
}

#[test]
fn empty_document() -> Result<()> {
    for input in ["", "\n", "  \n", "# c\n", "---\n"] {
        let doc = yaml::from_slice(input)?;
        assert!(is_null(Some(doc.as_ref())), "{input:?}");
        assert_eq!(doc.to_string(), input);
    }

    Ok(())
}

#[test]
fn empty_document_set() -> Result<()> {
    let mut doc = yaml::from_slice("---\n")?;
    doc.as_mut().set_u32(42);
    assert_eq!(doc.to_string(), "---\n42");
    Ok(())
}

#[test]
fn mapping_value_at_eof() -> Result<()> {
    for (input, key) in [
        ("a:\n", "a"),
        ("a:", "a"),
        ("a:  # c\n", "a"),
        ("a: 1\nb:\n", "b"),
    ] {
        let doc = yaml::from_slice(input)?;
        let root = doc.as_ref().as_mapping().context("missing mapping")?;
        assert!(is_null(root.get(key)), "{input:?}");
        assert_eq!(doc.to_string(), input);
    }

    Ok(())
}

#[test]
fn sequence_items() -> Result<()> {
    let cases: [(&str, &[bool]); 5] = [
        ("- a\n-\n", &[false, true]),
        ("- a\n-", &[false, true]),
        ("- a\n-\n- c\n", &[false, true, false]),
        ("-\n-\n- c\n", &[true, true, false]),
        ("-  # c\n\n- c\n", &[true, false]),
    ];

    for (input, nulls) in cases {
        let doc = yaml::from_slice(input)?;
        let root = doc.as_ref().as_sequence().context("missing sequence")?;
        assert_eq!(root.len(), nulls.len(), "{input:?}");

        for (index, &null) in nulls.iter().enumerate() {
            assert_eq!(is_null(root.get(index)), null, "{input:?}[{index}]");
        }

        assert_eq!(doc.to_string(), input);
    }

    Ok(())
}

#[test]
fn mapping_value_before_dedent() -> Result<()> {
    let input = "- a: 1\n  b:\n- c\n";
    let doc = yaml::from_slice(input)?;
    let root = doc.as_ref().as_sequence().context("missing sequence")?;
    assert_eq!(root.len(), 2);
    let first = root.get(0).and_then(|v| v.as_mapping()).context("first")?;
    assert!(is_null(first.get("b")));
    assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("c"));
    assert_eq!(doc.to_string(), input);
    Ok(())
}

#[test]
fn mapping_value_compact_sequence() -> Result<()> {
    let input = "a:\n- x\nb:\n";
    let doc = yaml::from_slice(input)?;
    let root = doc.as_ref().as_mapping().context("missing mapping")?;
    let a = root.get("a").and_then(|v| v.as_sequence()).context("a")?;
    assert_eq!(a.get(0).and_then(|v| v.as_str()), Some("x"));
    assert!(is_null(root.get("b")));
    assert_eq!(doc.to_string(), input);
    Ok(())
}

#[test]
fn set_empty_sequence_item() -> Result<()> {
    let mut doc = yaml::from_slice("- a\n-\n- c\n")?;
    let root = doc.as_ref().as_sequence().context("missing sequence")?;
    let id = root.get(1).context("missing item")?.id();
    doc.value_mut(id).set_u32(2);
    assert_eq!(doc.to_string(), "- a\n- 2\n- c\n");
    Ok(())
}

#[test]
fn set_empty_mapping_value() -> Result<()> {
    let mut doc = yaml::from_slice("a:\nb: 1\nc:\n")?;
    let root = doc.as_ref().as_mapping().context("missing mapping")?;
    let a = root.get("a").context("missing a")?.id();
    let c = root.get("c").context("missing c")?.id();
    doc.value_mut(a).set_u32(2);
    doc.value_mut(c).set_u32(3);
    assert_eq!(doc.to_string(), "a: 2\nb: 1\nc: 3\n");
    Ok(())
}
