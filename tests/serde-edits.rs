#![cfg(feature = "serde-edits")]

use anyhow::Result;
use nondestructive::yaml;

const INPUT: &str = "# c\na: 1 # x\nb: [1, 2]\nc:\n  - \"quoted\" # y\n  - 'single'\n";

#[test]
fn snapshot_json() -> Result<()> {
    let doc = yaml::from_slice(INPUT)?;
    let json = serde_json::to_string(&doc)?;
    let decoded: yaml::Document = serde_json::from_str(&json)?;
    assert_eq!(decoded.to_string(), INPUT);
    Ok(())
}

#[test]
fn snapshot_yaml() -> Result<()> {
    let doc = yaml::from_slice(INPUT)?;
    let yaml = serde_yaml::to_string(&doc)?;
    let decoded: yaml::Document = serde_yaml::from_str(&yaml)?;
    assert_eq!(decoded.to_string(), INPUT);
    Ok(())
}

#[test]
fn snapshot_after_edit() -> Result<()> {
    let mut doc = yaml::from_slice(INPUT)?;
    doc.as_mut()
        .into_mapping_mut()
        .expect("mapping")
        .insert_str("d", "new value");
    let expected = doc.to_string();

    let json = serde_json::to_string(&doc)?;
    let decoded: yaml::Document = serde_json::from_str(&json)?;
    assert_eq!(decoded.to_string(), expected);
    Ok(())
}
