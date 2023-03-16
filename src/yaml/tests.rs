use crate::yaml;

#[test]
fn nested_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = yaml::parse(
        r#"
        number1: 10
        number2: 20
        table:
            inner: 400
        string3: "I am a quoted string!"
        "#,
    )?;

    let mut root = doc
        .root_mut()
        .into_table_mut()
        .ok_or("missing root table")?;

    assert_eq!(
        root.as_ref().get("number1").and_then(|v| v.as_u32()),
        Some(10)
    );
    assert_eq!(
        root.as_ref().get("number2").and_then(|v| v.as_u32()),
        Some(20)
    );

    {
        let table = root
            .get_mut("table")
            .and_then(|v| v.into_table_mut())
            .ok_or("missing inner table")?;

        assert_eq!(
            table.as_ref().get("inner").and_then(|v| v.as_u32()),
            Some(400)
        );

        assert_ne!(
            table.as_ref().get("string3").and_then(|v| v.as_str()),
            Some("I am a quoted string!")
        );
    }

    assert_eq!(
        root.as_ref().get("string3").and_then(|v| v.as_str()),
        Some("I am a quoted string!")
    );

    Ok(())
}
