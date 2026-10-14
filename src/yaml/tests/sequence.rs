use alloc::string::ToString;

use anyhow::{Context, Result};

use crate::yaml;

#[test]
fn sequence() -> Result<()> {
    let doc = yaml::from_slice(
        r"
        - 1
        - 2
        - - 4
          - 5
        ",
    )?;

    assert_eq!(
        doc.to_string(),
        r"
        - 1
        - 2
        - - 4
          - 5
        ",
    );

    Ok(())
}

#[test]
fn inline_sequence() -> Result<()> {
    let doc = yaml::from_slice(
        r"
        - 1
        - 2
        - - [one, two, three]
          - 5
        ",
    )?;

    assert_eq!(
        doc.to_string(),
        r"
        - 1
        - 2
        - - [one, two, three]
          - 5
        ",
    );

    Ok(())
}

#[test]
fn mutable_nested_sequence() -> Result<()> {
    let mut doc = yaml::from_slice(
        r"
        - - 10
        ",
    )?;

    let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    let mut value = value
        .get_mut(0)
        .and_then(yaml::ValueMut::into_sequence_mut)
        .expect("missing inner");
    value.push_string("nice string");

    assert_eq!(
        doc.to_string(),
        r"
        - - 10
          - nice string
        "
    );

    Ok(())
}

#[test]
fn sequence_nth_back() -> Result<()> {
    let doc = yaml::from_slice("- 1\n- 2\n- 3\n")?;
    let seq = doc.as_ref().as_sequence().context("missing sequence")?;

    assert_eq!(seq.iter().nth_back(0).and_then(|v| v.as_u32()), Some(3));
    assert_eq!(seq.iter().nth(1).and_then(|v| v.as_u32()), Some(2));
    assert_eq!(seq.iter().rev().nth(1).and_then(|v| v.as_u32()), Some(2));

    let mut it = seq.iter();
    it.next_back();
    assert_eq!(it.nth_back(0).and_then(|v| v.as_u32()), Some(2));
    Ok(())
}

#[test]
fn remove_first() -> Result<()> {
    fn remove(input: &str, outer: Option<usize>, index: usize) -> Result<alloc::string::String> {
        let mut doc = yaml::from_slice(input)?;
        let mut value = doc.as_mut();

        if let Some(outer) = outer {
            value = value
                .into_sequence_mut()
                .context("missing sequence")?
                .get_into_mut(outer)
                .context("missing item")?;
        }

        let mut sequence = value.into_sequence_mut().context("missing sequence")?;
        assert!(sequence.remove(index));
        let mut out = alloc::vec::Vec::new();
        doc.write_to(&mut out)?;
        assert_eq!(out, doc.to_string().as_bytes());
        Ok(doc.to_string())
    }

    assert_eq!(remove("- 1\n- 2\n", None, 0)?, "- 2\n");
    assert_eq!(remove("- - 1\n  - 2\n- 3\n", Some(0), 0)?, "- - 2\n- 3\n");
    assert_eq!(remove("- 1 # one\n# two\n- 2\n", None, 0)?, "# two\n- 2\n");
    assert_eq!(remove("- 1\n- 2\n- 3\n", None, 1)?, "- 1\n- 3\n");
    assert_eq!(remove("[1, 2]", None, 0)?, "[2]");
    assert_eq!(remove("[ 1 , 2 ]", None, 0)?, "[ 2 ]");
    assert_eq!(remove("[\n  1,\n  2\n]", None, 0)?, "[\n  2\n]");
    assert_eq!(remove("[1 , 2 ,]", None, 0)?, "[2 ,]");
    Ok(())
}

/// Insert `x` at `index` into the root sequence of `input`, check the output
/// and that it reads back with `x` at `index`.
fn insert_case(input: &str, index: usize, expected: &str) -> Result<()> {
    let mut doc = yaml::from_slice(input)?;
    let mut root = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    let before = root
        .as_ref()
        .iter()
        .map(|v| v.to_string())
        .collect::<alloc::vec::Vec<_>>();
    root.insert_string(index, "x");
    assert_eq!(doc.to_string(), expected, "insert {index} into {input:?}");

    let doc = yaml::from_slice(expected)?;
    let root = doc.as_ref().as_sequence().context("not a sequence")?;
    let mut after = root
        .iter()
        .map(|v| v.to_string())
        .collect::<alloc::vec::Vec<_>>();
    assert_eq!(after.remove(index), "x");
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn insert_block() -> Result<()> {
    let input = "\n- a\n- b\n- c\n";
    insert_case(input, 0, "\n- x\n- a\n- b\n- c\n")?;
    insert_case(input, 1, "\n- a\n- x\n- b\n- c\n")?;
    insert_case(input, 3, "\n- a\n- b\n- c\n- x\n")?;

    insert_case("- a\n", 0, "- x\n- a\n")?;
    insert_case("- a\r\n- b\r\n", 1, "- a\r\n- x\r\n- b\r\n")?;
    insert_case("- a\r\n- b\r\n", 0, "- x\r\n- a\r\n- b\r\n")?;

    let input = "key:\n  - a\n  # about b\n  - b\n";
    let mut doc = yaml::from_slice(input)?;
    let mut root = doc.as_mut().into_mapping_mut().context("mapping")?;
    let mut seq = root
        .get_mut("key")
        .and_then(yaml::ValueMut::into_sequence_mut)
        .context("sequence")?;
    seq.insert_u32(0, 1);
    seq.insert_u32(2, 2);
    seq.insert_u32(4, 3);
    assert_eq!(
        doc.to_string(),
        "key:\n  - 1\n  - a\n  - 2\n  # about b\n  - b\n  - 3\n"
    );
    Ok(())
}

#[test]
fn insert_block_comments() -> Result<()> {
    let input = "# head\n- a # one\n\n# about b\n- b\n";
    insert_case(input, 0, "# head\n- x\n- a # one\n\n# about b\n- b\n")?;
    insert_case(input, 1, "# head\n- a # one\n- x\n\n# about b\n- b\n")?;
    insert_case(input, 2, "# head\n- a # one\n\n# about b\n- b\n- x\n")?;
    Ok(())
}

#[test]
fn insert_flow() -> Result<()> {
    let input = "[a, b, c]";
    insert_case(input, 0, "[x, a, b, c]")?;
    insert_case(input, 1, "[a, x, b, c]")?;
    insert_case(input, 3, "[a, b, c, x]")?;

    insert_case("[a,b]", 0, "[x,a,b]")?;
    insert_case("[a,b]", 1, "[a,x,b]")?;
    insert_case("[ a , b ]", 0, "[ x , a , b ]")?;
    insert_case("[ a , b ]", 1, "[ a , x , b ]")?;
    insert_case("[ a , b ]", 2, "[ a , b, x ]")?;
    insert_case("[a]", 0, "[x, a]")?;
    insert_case("[ a ]", 0, "[ x, a ]")?;
    insert_case("[a, b,]", 0, "[x, a, b,]")?;
    insert_case("[a, b,]", 2, "[a, b, x,]")?;
    insert_case("[a ,]", 0, "[x , a ,]")?;
    insert_case("[a, # c\n b]", 1, "[a, # c\n x,\n b]")?;
    Ok(())
}

#[test]
fn insert_flow_multiline() -> Result<()> {
    let input = "[\n  a,\n  # about b\n  b,\n]";
    insert_case(input, 0, "[\n  x,\n  a,\n  # about b\n  b,\n]")?;
    insert_case(input, 1, "[\n  a,\n  x,\n  # about b\n  b,\n]")?;
    insert_case(input, 2, "[\n  a,\n  # about b\n  b,\n  x,\n]")?;

    let input = "[ # first\n  a,\n  b\n]";
    insert_case(input, 0, "[ # first\n  x,\n  a,\n  b\n]")?;
    insert_case(input, 1, "[ # first\n  a,\n  x,\n  b\n]")?;
    insert_case(input, 2, "[ # first\n  a,\n  b,\n  x\n]")?;
    Ok(())
}

#[test]
#[should_panic = "insertion index (is 2) should be <= len (is 1)"]
fn insert_out_of_bounds() {
    let mut doc = yaml::from_slice("[a]").unwrap();
    let mut root = doc.as_mut().into_sequence_mut().unwrap();
    root.insert_bool(2, true);
}

#[test]
fn document_start_after_sequence_is_an_error() {
    for input in ["- 1\n---\n- 2", "- 1\n---", "- 1\n--- \n", "- 1\n---\t- 2"] {
        let err = yaml::from_slice(input).unwrap_err();
        assert_eq!(*err.kind(), yaml::ErrorKind::ExpectedEof, "{input:?}");
    }

    // Same error as for a `---` after a mapping entry.
    for input in [
        "a:\n- 1\n---\n- 2",
        "a:\n  - 1\n---\n  - 2",
        "a: 1\n---\nb: 2",
    ] {
        let err = yaml::from_slice(input).unwrap_err();
        assert_eq!(
            *err.kind(),
            yaml::ErrorKind::BadMappingSeparator,
            "{input:?}"
        );
    }
}

#[test]
fn dashes_in_items() -> Result<()> {
    let input = "- ---x\n- --\n- ---\n";
    let doc = yaml::from_slice(input)?;
    assert_eq!(doc.to_string(), input);

    let root = doc
        .as_ref()
        .as_sequence()
        .context("missing root sequence")?;
    assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("---x"));
    assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("--"));
    assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("---"));

    Ok(())
}
