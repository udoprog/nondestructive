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
