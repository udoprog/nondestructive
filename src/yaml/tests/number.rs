use anyhow::Result;

use crate::yaml;

#[test]
fn keywords_are_case_strict() -> Result<()> {
    for (input, expected) in [
        ("true", Some(true)),
        ("True", Some(true)),
        ("TRUE", Some(true)),
        ("false", Some(false)),
        ("False", Some(false)),
        ("FALSE", Some(false)),
        ("tRuE", None),
        ("fALSE", None),
    ] {
        let doc = yaml::from_slice(input)?;
        assert_eq!(doc.as_ref().as_bool(), expected, "{input}");
    }

    for input in ["null", "Null", "NULL", "~"] {
        let doc = yaml::from_slice(input)?;
        assert!(matches!(doc.as_ref().as_any(), yaml::Any::Null), "{input}");
    }

    let doc = yaml::from_slice("nUlL")?;
    assert_eq!(doc.as_ref().as_str(), Some("nUlL"));
    Ok(())
}

#[test]
fn core_schema_numbers() -> Result<()> {
    for (input, expected) in [
        ("0x1F", 31),
        ("0xff", 255),
        ("0o17", 15),
        ("+1", 1),
        ("0", 0),
    ] {
        let doc = yaml::from_slice(input)?;
        let value = doc.as_ref();
        assert!(value.as_number().is_some(), "{input}");
        assert_eq!(value.as_u32(), Some(expected), "{input}");
        assert_eq!(value.as_i64(), Some(i64::from(expected)), "{input}");
        assert_eq!(value.as_f64(), Some(f64::from(expected)), "{input}");
    }

    let doc = yaml::from_slice("-5")?;
    assert_eq!(doc.as_ref().as_i8(), Some(-5));
    assert_eq!(doc.as_ref().as_u8(), None);

    for (input, expected) in [
        (".inf", f64::INFINITY),
        (".Inf", f64::INFINITY),
        ("+.INF", f64::INFINITY),
        ("-.inf", f64::NEG_INFINITY),
        ("+1.5", 1.5),
        ("+.5", 0.5),
        ("-1e3", -1000.0),
        ("007.5", 7.5),
    ] {
        let doc = yaml::from_slice(input)?;
        assert_eq!(doc.as_ref().as_f64(), Some(expected), "{input}");
        assert_eq!(doc.as_ref().as_u32(), None, "{input}");
    }

    for input in [".nan", ".NaN", ".NAN"] {
        let doc = yaml::from_slice(input)?;
        assert!(doc.as_ref().as_f32().is_some_and(f32::is_nan), "{input}");
    }

    Ok(())
}

#[test]
fn number_like_strings() -> Result<()> {
    for input in [
        "007", "-007", "00", "0x", "0x1G", "0X1F", "0o18", "+0x1F", "-0x1F", "0b101", "+-1", "++1",
        ".iNf", "-.nan", "+.nan", ".", "+", "1_000", "1e", "+.e1",
    ] {
        let doc = yaml::from_slice(input)?;
        assert_eq!(doc.as_ref().as_str(), Some(input), "{input}");
    }

    Ok(())
}
