//! Support for non-destructive YAML editing.
//!
//! YAML is parsed with [`from_slice`], which returns a [`Document`]. Documents
//! are serialized through their [`std::fmt::Display`] and
//! [`Document::write_to`] implementations.
//!
//! With `serde` support enabled, a [`Document`] can be deserialized through
//! [`serde`] and a [`Value`] can be both serialized and deserialized.
//!
//! ## Specification compliance
//!
//! This parser does not strictly adhere to the [YAML specification].
//!
//! In particular:
//! * We support any form of indentation, not just spaces.
//! * Text after a block scalar header (`|` or `>`) on the same line is taken
//!   as its first line.
//! * Input is not required to be UTF-8. [`Document::write_to`] reproduces
//!   the bytes exactly, but [`Display`][std::fmt::Display] replaces invalid
//!   UTF-8 with `U+FFFD`, so `to_string()` is lossy for such documents (and
//!   fails the `nondestructive_write_to_eq` assertion).
//! * Anchors (`&a`), aliases (`*a`) and tags (`!!str`, `!foo`) are not
//!   supported. They are read as part of the plain scalar they precede, so
//!   `a: &x 1` has the string value `"&x 1"` and `b: *x` the string `"*x"`.
//! * Only one document is supported. A leading `---` and directives such as
//!   `%YAML 1.2` are accepted, but a document end marker (`...`) is an error,
//!   and so is a `---` after the root value.
//! * Explicit `?` keys are not supported. They are an error in block
//!   mappings, while in a flow mapping `{? a: 1}` has the key `"? a"`.
//! * A plain scalar at the root must fit on a single line, so `one\ntwo` is
//!   an error. Multi-line plain scalars work as mapping values and sequence
//!   items.
//! * Duplicate keys in a mapping are not rejected.
//! * Plain keys in [Mappings][Mapping] can be anything on a single line, the
//!   only requirement is that they are succeeded by a colon (`:`) followed by
//!   whitespace. Quoted keys may be followed directly by a colon in block
//!   mappings too, so `"a":1` is accepted.
//! * [Sequences][Sequence] can also be anything, everything after the `-` is
//!   used as a value.
//! * Every key in a flow mapping must be followed by a colon, so `{a, b: 1}` is
//!   rejected, and flow sequences cannot contain single-pair mappings like
//!   `[a: 1]`.
//! * Plain scalars resolve as in the YAML 1.2 core schema (`null`, `Null`,
//!   `NULL` and `~`; `true`/`True`/`TRUE` and the same for `false`; decimal,
//!   `0x` and `0o` integers, floats, `.inf` and `.nan`), except that, like
//!   libyaml, a decimal integer with a leading zero such as `007` is a string.
//!
//! This means that we will validly parse both spec and non-spec compliant YAML.
//! The key here is that editing performed by this crate is non-destructive. So
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
//! Serde is supported through the `serde`
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
pub(crate) use self::parsing::Parser;

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
#[cfg_attr(doc_cfg, doc(cfg(feature = "serde")))]
pub mod serde;

#[cfg(any(feature = "serde", feature = "serde-edits", feature = "musli02"))]
mod serde_hint {
    #[cfg(feature = "serde-edits")]
    use serde::{Deserialize, Serialize};

    /// A number hint associated with a deserialized number.
    #[derive(Debug, Clone, Copy)]
    #[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "serde-edits", serde(tag = "kind"))]
    #[cfg_attr(
        feature = "musli02",
        derive(musli_core02::Encode, musli_core02::Decode),
        musli(crate = musli_core02)
    )]
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

#[cfg(not(any(feature = "serde", feature = "serde-edits", feature = "musli02")))]
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
