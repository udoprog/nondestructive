//! Support for non-destructive TOML editing.

mod error;
pub use self::error::{Error, ErrorKind};

mod parsing;
pub use self::parsing::Parser;

mod document;
pub use self::document::Document;

mod value;
pub use self::value::Value;

mod value_mut;
pub use self::value_mut::ValueMut;

mod table;
pub use self::table::Table;

mod raw;

mod data;
pub use self::data::Id;

/// Parse a TOML [`Document`].
///
/// # Errors
///
/// Errors in case the document cannot be parsed as TOML.
pub fn from_slice<D>(input: D) -> Result<Document, Error>
where
    D: AsRef<[u8]>,
{
    let parser = Parser::new(input.as_ref());
    parser.parse()
}
