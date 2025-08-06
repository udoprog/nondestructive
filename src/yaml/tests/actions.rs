use anyhow::Result;

use crate::yaml;

/// Just roundtrip a fairly complex document to ensure it's correctly processed.
#[test]
#[ignore = "Doesn't work as intended for now"]
fn actions() -> Result<()> {
    const ACTION: &str = include_str!("actions.yaml");
    let doc = yaml::from_slice(ACTION)?;
    assert_eq!(doc.to_string(), ACTION);
    Ok(())
}
