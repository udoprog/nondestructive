//! Support for non-destructive YAML editing.
//!
//! YAML is parsed with [`from_slice`], which returns a [`Document`]. Documents
//! are serialized through their [`std::fmt::Display`] and
//! [`Document::write_to`] implementations.
//!
//! With `serde` support enabled, [`Document`] can also be serialized through
//! [`serde`].
//!
//! ## Specification compliance
//!
//! This parser does not strictly adhere to the [YAML specification].
//!
//! In particular:
//! * We support any form of indentation, not just spaces.
//! * Neither input nor output is not required to be UTF-8.
//! * Keys in [Mappings][Mapping] can be anything, the only requirement is that
//!   they are succeeded by a colon (`:`).
//! * [Sequences][Sequence] can also be anything, everything after the `-` is
//!   used as a value.
//!
//! This means that we will validly parse both spec and non-spec compliant YAML.
//! They key here is that editing performed by this crate is non-destructive. So
//! if the source is spec compliant YAML, then we will produce spec compliant
//! YAML. If the source is **not** spec compliant YAML we will produce similarly
//! non-spec compliant YAML.
//!
//! If you want to ensure that valid YAML is produced, we recommend that you
//! enable the `serde` feature and use a crate such [`serde-yaml`]. But keep in
//! mind that it will not preserve the original structure of the document. See
//! the [`serde` module][serde] for how this is done.
//!
//! <br>
//!
//! ## Serde support
//!
//! Serde is supported for [`Document`] and [`Value`] through the `serde`
//! feature, see the [`serde` module][serde] for documentation.
//!
//! # Examples
//!
//! ```
//! use anyhow::Context;
//! use nondestructive::yaml;
//!
//! let doc = yaml::from_slice("32")?;
//! assert_eq!(doc.as_ref().as_u32(), Some(32));
//!
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! [`serde-yaml`]: https://docs.rs/serde_yaml
//! [YAML specification]: https://yaml.org/

#[cfg(test)]
mod tests;

#[macro_use]
mod parsing;
pub use self::parsing::Parser;

mod any;
pub use self::any::Any;

mod any_mut;
pub use self::any_mut::AnyMut;

mod data;
pub use self::data::Id;

mod error;
pub use self::error::{Error, ErrorKind};

mod document;
pub use self::document::Document;

mod raw;

mod value;
pub use self::value::{Block, Chomp, Null, Separator, StringKind, Value};

mod value_mut;
pub use self::value_mut::ValueMut;

mod number;
pub use self::number::Number;

mod string;
pub use self::string::String;

pub mod sequence;
#[doc(inline)]
pub use self::sequence::{Sequence, SequenceMut};

pub mod mapping;
#[doc(inline)]
pub use self::mapping::{Mapping, MappingMut};

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub mod serde;

#[cfg(any(feature = "serde", feature = "serde-edits"))]
mod serde_hint {
    #[cfg(feature = "serde-edits")]
    use serde::{Deserialize, Serialize};

    /// A number hint associated with a deserialized number.
    #[derive(Debug, Clone, Copy)]
    #[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "serde-edits", serde(tag = "kind"))]
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
}

#[cfg(not(any(feature = "serde", feature = "serde-edits")))]
mod serde_hint {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    #[non_exhaustive]
    pub(crate) struct RawNumberHint;

    pub(crate) const F32: RawNumberHint = RawNumberHint;
    pub(crate) const F64: RawNumberHint = RawNumberHint;
    pub(crate) const U8: RawNumberHint = RawNumberHint;
    pub(crate) const U16: RawNumberHint = RawNumberHint;
    pub(crate) const U32: RawNumberHint = RawNumberHint;
    pub(crate) const U64: RawNumberHint = RawNumberHint;
    pub(crate) const U128: RawNumberHint = RawNumberHint;
    pub(crate) const I8: RawNumberHint = RawNumberHint;
    pub(crate) const I16: RawNumberHint = RawNumberHint;
    pub(crate) const I32: RawNumberHint = RawNumberHint;
    pub(crate) const I64: RawNumberHint = RawNumberHint;
    pub(crate) const I128: RawNumberHint = RawNumberHint;
}

/// Parse a YAML document.
///
/// # Errors
///
/// Errors in case the document cannot be parsed as YAML.
pub fn from_slice<D>(input: D) -> Result<Document, Error>
where
    D: AsRef<[u8]>,
{
    let parser = Parser::new(input.as_ref());
    parser.parse()
}
