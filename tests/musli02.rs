#![cfg(feature = "musli02")]

use anyhow::{Context, Result};
use nondestructive::yaml;

const INPUT: &str = "# c\na: 1 # x\nb: [1, 2]\nc:\n  - \"quoted\" # y\n  - 'single'\nd: ~\ne: true\nf: |\n  block\n";

fn round_trip<T>(value: &T) -> Result<T>
where
    T: musli02::Encode<musli02::mode::Binary>
        + for<'de> musli02::Decode<'de, musli02::mode::Binary, musli02::alloc::Global>,
{
    let bytes = musli02::storage::to_vec(value)?;
    Ok(musli02::storage::from_slice(&bytes)?)
}

#[test]
fn unmodified() -> Result<()> {
    let doc = yaml::from_slice(INPUT)?;
    let decoded = round_trip(&doc)?;
    assert_eq!(decoded.to_string(), INPUT);
    Ok(())
}

#[test]
fn edited_with_id() -> Result<()> {
    let mut doc = yaml::from_slice(INPUT)?;

    let mut root = doc.as_mut().into_mapping_mut().context("mapping")?;
    // Leaves a vacant slot in the node storage.
    assert!(root.remove("a"));
    root.insert_string("g", "new value");
    root.insert_u32("h", 42);

    let id = doc
        .as_ref()
        .as_mapping()
        .and_then(|m| m.get("c"))
        .context("missing c")?
        .id();

    let expected = doc.to_string();

    let mut decoded = round_trip(&doc)?;
    assert_eq!(decoded.to_string(), expected);
    assert_eq!(round_trip(&id)?, id);

    let c = decoded.value(id).as_sequence().context("sequence")?;
    assert_eq!(c.get(1).and_then(|v| v.as_str()), Some("single"));

    decoded
        .value_mut(id)
        .into_sequence_mut()
        .context("sequence")?
        .push_string("pushed");

    doc.value_mut(id)
        .into_sequence_mut()
        .context("sequence")?
        .push_string("pushed");

    assert_eq!(decoded.to_string(), doc.to_string());
    Ok(())
}
