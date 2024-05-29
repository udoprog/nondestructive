use anyhow::{Context, Result};
use nondestructive::yaml;

#[test]
fn push_mapping() -> Result<()> {
    let mut doc = yaml::from_slice(
        r"
        - one
        - two
        ",
    )?;

    let mut seq = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    let mut mapping = seq.push(yaml::Separator::Auto).make_mapping();

    mapping.insert_u32("three", 3);
    mapping.insert_u32("four", 4);
    let mut mapping2 = mapping.insert("five", yaml::Separator::Auto).make_mapping();

    mapping2.insert_str("six", "six");
    mapping2.insert_str("seven", "seven");

    assert_eq!(
        doc.to_string(),
        r"
        - one
        - two
        - three: 3
          four: 4
          five:
            six: six
            seven: seven
        "
    );

    Ok(())
}
