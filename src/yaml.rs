//! Support for non-destructive YAML editing.
//!
//! YAML is parsed with [parse], which returns a [Document].
//!
//! ## Specification Compliance
//!
//! This parser does not strictly adhere to the YAML 1.2 specification.
//!
//! In particular:
//! * We support any form of indentation.
//! * Input is not required to be UTF-8.
//! * Keys in [Tables][Table] can be anything, the only requirement is that they
//!   are succeeded by a colon (`:`).
//! * [List] items can be anything, everything after the `-` is consumed.
//!
//! This means that we will validly parse both spec and non-spec compliant YAML.
//! They key here is that editing performed by this crate is non-destructive. So
//! if the source is spec compliant YAML we will produce spec compliant YAML, if
//! the source is **not** spec compliant YAML we will produce the same non-spec
//! compliant YAML.
//!
//! If you want to produce valid YAML, we recommend that you enable the `serde`
//! feature and make use of the fact that [`Document`] implements [`Serialize`]
//! and [`IntoDeserializer`][serde::IntoDeserializer].
//!
//! ## Serde support
//!
//! By enabling the `serde` feature [`Document`] implements
//! [Serialize][serde::Serialize] and
//! [`IntoDeserializer`][serde::IntoDeserializer], allowing it to be used to
//! deserialize into types:
//!
//! ```
//! use nondestructive::yaml;
//! use serde::Deserialize;
//! use serde::de::IntoDeserializer;
//!
//! const SOURCE: &str = r#"
//! name: Descartes
//! year: 1596
//! books:
//! - year: 1618
//!   title: Musicae Compendium
//! - year: 1628
//!   title: Regulae ad directionem ingenii
//! - year: 1630
//!   title: De solidorum elementis
//! - year: 1631
//!   title: La recherche de la vérité par la lumière naturelle
//! - year: 1633
//!   title: Le Monde and L'Homme
//! - year: 1637
//!   title: Discours de la méthode
//! - year: 1637
//!   title: La Géométrie
//! - year: 1641
//!   title: Meditationes de prima philosophia
//! - year: 1644
//!   title: Principia philosophiae
//! - year: 1647
//!   title: Notae in programma
//! - year: 1648
//!   title: La description du corps humain
//! - year: 1648
//!   title: Responsiones Renati Des Cartes...
//! - year: 1649
//!   title: Les passions de l'âme
//! - year: 1657
//!   title: Correspondance
//! "#;
//!
//! let doc = yaml::from_bytes(SOURCE)?;
//!
//! let string = serde_yaml::to_string(&doc)?;
//! assert_eq!(string.trim(), SOURCE.trim());
//!
//! #[derive(Deserialize)]
//! struct Book<'a> {
//!     title: &'a str,
//!     year: u32,
//! }
//!
//! #[derive(Deserialize)]
//! struct Record<'a> {
//!     name: &'a str,
//!     year: u32,
//!     #[serde(borrow)]
//!     books: Vec<Book<'a>>,
//! }
//!
//! let record = Record::deserialize(doc.into_deserializer())?;
//! assert_eq!(record.name, "Descartes");
//! assert_eq!(record.year, 1596);
//! assert_eq!(record.books.len(), 14);
//!
//! for book in &record.books {
//!     assert!(book.year >= 1600 && book.year < 1700);
//! }
//!
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! # Examples
//!
//! ```
//! use nondestructive::yaml;
//!
//! let doc = yaml::from_bytes("32")?;
//! assert_eq!(doc.root().as_u32(), Some(32));
//!
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```

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
pub use self::value::{NullKind, Separator, Value};

mod value_mut;
pub use self::value_mut::ValueMut;

pub mod list;
pub use self::list::{List, ListMut};

pub mod table;
pub use self::table::{Table, TableMut};

#[cfg(feature = "serde")]
mod serde;

#[cfg(not(feature = "serde"))]
mod serde {
    #[repr(transparent)]
    #[non_exhaustive]
    pub(crate) struct RawNumberHint;
    pub(crate) const FLOAT_32: RawNumberHint = RawNumberHint;
    pub(crate) const FLOAT_64: RawNumberHint = RawNumberHint;
    pub(crate) const UNSIGNED_8: RawNumberHint = RawNumberHint;
    pub(crate) const UNSIGNED_16: RawNumberHint = RawNumberHint;
    pub(crate) const UNSIGNED_32: RawNumberHint = RawNumberHint;
    pub(crate) const UNSIGNED_64: RawNumberHint = RawNumberHint;
    pub(crate) const UNSIGNED_128: RawNumberHint = RawNumberHint;
    pub(crate) const SIGNED_8: RawNumberHint = RawNumberHint;
    pub(crate) const SIGNED_16: RawNumberHint = RawNumberHint;
    pub(crate) const SIGNED_32: RawNumberHint = RawNumberHint;
    pub(crate) const SIGNED_64: RawNumberHint = RawNumberHint;
    pub(crate) const SIGNED_128: RawNumberHint = RawNumberHint;
}

/// Parse a YAML document.
///
/// # Errors
///
/// Errors in case the document cannot be parsed as YAML.
pub fn from_bytes<D>(input: D) -> Result<Document, Error>
where
    D: AsRef<[u8]>,
{
    let parser = Parser::new(input.as_ref());
    parser.parse()
}
