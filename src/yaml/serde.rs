//! Serde support for YAML.
//!
//! By enabling the `serde` feature [`Document`] implements [`Serialize`] and
//! [`IntoDeserializer`], allowing it to be used to deserialize into types:
//!
//! [`Serialize`]: serde::Serialize
//! [`IntoDeserializer`]: serde::de::IntoDeserializer
//! [`Document`]: crate::yaml::Document
//!
//! ```
//! use anyhow::Context;
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
//! let doc = yaml::from_slice(SOURCE)?;
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

mod de;
mod error;
mod ser;

pub use self::error::Error;

/// A number hint associated with a deserialized number.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RawNumberHint {
    /// A 32-bit float.
    Float32,
    /// A 64-bit float.
    Float64,
    /// An unsigned 8-bit number.
    Unsigned8,
    /// An unsigned 16-bit number.
    Unsigned16,
    /// An unsigned 32-bit number.
    Unsigned32,
    /// An unsigned 64-bit number.
    Unsigned64,
    /// An unsigned 128-bit number.
    Unsigned128,
    /// A signed 8-bit number.
    Signed8,
    /// A signed 16-bit number.
    Signed16,
    /// A signed 32-bit number.
    Signed32,
    /// A signed 64-bit number.
    Signed64,
    /// A signed 128-bit number.
    Signed128,
}

pub(crate) const F32: RawNumberHint = RawNumberHint::Float32;
pub(crate) const F64: RawNumberHint = RawNumberHint::Float64;
pub(crate) const U8: RawNumberHint = RawNumberHint::Unsigned8;
pub(crate) const U16: RawNumberHint = RawNumberHint::Unsigned16;
pub(crate) const U32: RawNumberHint = RawNumberHint::Unsigned32;
pub(crate) const U64: RawNumberHint = RawNumberHint::Unsigned64;
pub(crate) const U128: RawNumberHint = RawNumberHint::Unsigned128;
pub(crate) const I8: RawNumberHint = RawNumberHint::Signed8;
pub(crate) const I16: RawNumberHint = RawNumberHint::Signed16;
pub(crate) const I32: RawNumberHint = RawNumberHint::Signed32;
pub(crate) const I64: RawNumberHint = RawNumberHint::Signed64;
pub(crate) const I128: RawNumberHint = RawNumberHint::Signed128;
