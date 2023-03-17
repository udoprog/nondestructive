//! Support for non-destructive YAML editing.
//!
//! YAML is parsed with [parse], which returns a [Document].
//!
//! # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::parse("32")?;
/// assert_eq!(doc.root().as_u32(), Some(32));
///
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```

#[cfg(test)]
mod tests;

#[macro_use]
mod parsing;
pub use self::parsing::Parser;

mod error;
pub use self::error::Error;

mod document;
pub use self::document::Document;

mod raw;

mod value;
pub use self::value::{List, NullKind, Separator, StringKind, Table, Value};

mod value_mut;
pub use self::value_mut::{ListMut, TableMut, ValueMut};

/// Parse a YAML document.
pub fn parse<D>(input: D) -> Result<Document, Error>
where
    D: AsRef<[u8]>,
{
    let parser = Parser::new(input.as_ref());
    parser.parse()
}
