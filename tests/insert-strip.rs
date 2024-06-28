use anyhow::{Context, Result};
use nondestructive::yaml;

#[test]
fn insert_strip1() -> Result<()> {
    let mut doc = yaml::from_slice("doc: test")?;

    let b_ty = yaml::Block::Literal(yaml::Chomp::Strip);

    let mut root = doc.as_mut().into_mapping_mut().context("missing root")?;
    root.insert_block("my_string", ["one", "two"], b_ty);

    assert_eq!(doc.to_string(), "doc: test\nmy_string: |-\n  one\n  two");
    Ok(())
}

#[test]
fn insert_strip2() -> Result<()> {
    let mut doc = yaml::from_slice("\ndoc: test")?;

    let b_ty = yaml::Block::Literal(yaml::Chomp::Strip);

    let mut root = doc.as_mut().into_mapping_mut().context("missing root")?;
    root.insert_block("my_string", ["one", "two"], b_ty);

    println!("{doc}");
    assert_eq!(doc.to_string(), "\ndoc: test\nmy_string: |-\n  one\n  two");
    Ok(())
}

#[test]
fn insert_strip_varying() -> Result<()> {
    let mut doc = yaml::from_slice(
        "
    doc: test
    obj:
      title: Object
    obj_array:
      - title: Object in an array",
    )?;

    let b_ty = yaml::Block::Literal(yaml::Chomp::Strip);

    let mut root = doc.as_mut().into_mapping_mut().context("missing root")?;
    root.insert_block("my_string_outer", ["one", "two"], b_ty);

    let mut obj = doc
        .as_mut()
        .into_mapping_mut()
        .context("missing root")?
        .get_into_mut("obj")
        .context("missing root.obj")?
        .into_mapping_mut()
        .context("not an object")?;

    obj.insert_block("my_string_inner1", ["one", "two"], b_ty);

    let mut obj_arr = doc
        .as_mut()
        .into_mapping_mut()
        .context("missing root")?
        .get_into_mut("obj_array")
        .context("missing root.obj_array")?
        .into_sequence_mut()
        .context("not an array")?
        .get_into_mut(0)
        .context("no index 0")?
        .into_mapping_mut()
        .context("not an object")?;

    obj_arr.insert_block("my_string_inner2", ["one", "two"], b_ty);

    assert_eq!(
        doc.to_string(),
        "
    doc: test
    obj:
      title: Object
      my_string_inner1: |-
        one
        two
    obj_array:
      - title: Object in an array
        my_string_inner2: |-
          one
          two
    my_string_outer: |-
      one
      two"
    );
    Ok(())
}
