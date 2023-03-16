use crate::yaml;

#[test]
fn test_property_eol() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = yaml::parse(
        r#"
        table:
            inner: so this is as a matter of @ course, a large document
        string3: "I am a quoted string!"
        "#,
    )?;

    let mut root = doc
        .root_mut()
        .into_table_mut()
        .ok_or("missing root table")?;

    let table = root
        .get_mut("table")
        .and_then(|v| v.into_table_mut())
        .ok_or("missing inner table")?;

    let table = table.as_ref();
    let string = table.get("inner").and_then(|v| v.as_str());
    assert_eq!(
        string,
        Some("so this is as a matter of @ course, a large document")
    );
    Ok(())
}
