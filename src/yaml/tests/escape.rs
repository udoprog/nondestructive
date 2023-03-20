use anyhow::Result;

use crate::yaml;

macro_rules! test {
    ($from:expr, $to:expr) => {
        let doc = yaml::from_slice($from)?;
        assert_eq!(doc.as_ref().as_str(), Some($to));
    };
}

#[test]
fn double_quotes() -> Result<()> {
    test!("\"a \\n b\"", "a \n b");
    test!("\"a \\0 b\"", "a \x00 b");
    test!("\"a \\a b\"", "a \x07 b");
    test!("\"a \\b b\"", "a \x08 b");
    test!("\"a \\t b\"", "a \x09 b");
    test!("\"a \\v b\"", "a \x0b b");
    test!("\"a \\f b\"", "a \x0c b");
    test!("\"a \\r b\"", "a \r b");
    test!("\"a \\e b\"", "a \x1b b");
    test!("\"a \\\\ b\"", "a \" b");

    test!("\"a \\x77 b\"", "a \x77 b");
    test!("\"a \\u79c1 b\"", "a 私 b");
    Ok(())
}

#[test]
fn single_quotes() -> Result<()> {
    test!("'It is a party!'", "It is a party!");
    test!("'It''s a party!'", "It's a party!");
    Ok(())
}
