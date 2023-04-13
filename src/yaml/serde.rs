//! Serde support for YAML.
//!
//! There are two serde related features available in this crate:
//! * `serde` which allows original serialization to happen.
//! * `serde-edits` which allows a true representation of the current state of a
//!   [`Document`] to happen, allowing for the saving of a snapshot of the
//!   document to be later decoded.
//!
//! By enabling the `serde` feature [`Value`] implements [`Serialize`] and
//! [`IntoDeserializer`], allowing it to be used to deserialize into types.
//!
//! [`Serialize`]: serde::Serialize
//! [`IntoDeserializer`]: serde::de::IntoDeserializer
//! [`Value`]: crate::yaml::Value
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
//! let string = serde_yaml::to_string(&doc.as_ref())?;
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
