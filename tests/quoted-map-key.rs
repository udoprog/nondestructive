use anyhow::{Context, Result};
use nondestructive::yaml;

#[test]
fn quoted_map_key() -> Result<()> {
    let doc = yaml::from_slice("'!quoted_keys': |-\n  are compliant")?;

    let root = doc.as_ref().as_mapping().context("missing root")?;
    let value = root.get("!quoted_keys").and_then(|v| v.as_str());

    assert_eq!(value, Some("are compliant"));
    Ok(())
}
