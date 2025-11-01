//! [<img alt="github" src="https://img.shields.io/badge/github-udoprog/nondestructive-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/nondestructive)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/nondestructive.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/nondestructive)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-nondestructive-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/nondestructive)
//!
//! Nondestructive editing tries to preserve as much as possible of the existing
//! structure, while allowing the document to be modified in place.
//!
//! **This project is as of yet, incomplete!**
//!
//! See the corresponding module for detailed documentation on how to use:
//!
//! * [YAML support][yaml].
//!
//! <br>
//!
//! ## Working with documents
//!
//! A document is deserialized into a large object from which all access and
//! mutation must occur. This makes the API a bit harder to use than your
//! typical serialization library but is necessary to ensure that it has access
//! to all the necessary data to perform nondestructive editing.
//!
//! One particular complication is that each method which provides mutable
//! access to the document needs to have two variations: `as_<something>_mut`
//! and `into_<something>_mut`. If we look at [`ValueMut::as_mapping_mut`] and
//! [`ValueMut::into_mapping_mut`] the difference is that the latter consumes
//! `self` and returns a [`ValueMut`] with its associated lifetime.
//!
//! <br>
//!
//! ```
//! use anyhow::Context;
//! use nondestructive::yaml;
//!
//! let mut doc = yaml::from_slice(
//!     r"
//!     greeting: Hello World!
//!     "
//! )?;
//!
//! // Access through the document:
//! assert_eq!(
//!     doc.as_ref().as_mapping().and_then(|m| m.get("greeting")?.as_str()),
//!     Some("Hello World!")
//! );
//!
//! // Mutation through the document:
//! let mut mapping = doc.as_mut().into_mapping_mut().context("missing root mapping")?;
//! mapping.insert_str("greeting2", "Hello Rust!");
//!
//! assert_eq!(
//!     doc.to_string(),
//!     r"
//!     greeting: Hello World!
//!     greeting2: Hello Rust!
//!     "
//! );
//! # Ok::<_, anyhow::Error>(())
//! ```
//!
//! <br>
//!
//! If we were to change the line which uses `into_mapping_mut` we would get a
//! lifetime error:
//!
//! <br>
//!
//! ```text
//! error[E0716]: temporary value dropped while borrowed
//!   --> src\lib.rs:43:19
//!    |
//! 20 | let mut mapping = doc.as_mut().as_mapping_mut().context("missing root mapping")?;
//!    |                   ^^^^^^^^^^^^                                                  - temporary value is freed at the end of this statement
//!    |                   |
//!    |                   creates a temporary value which is freed while still in use
//! 21 | mapping.insert_str("greeting2", "Hello Rust!");
//!    | ---------------------------------------------- borrow later used here
//!    |
//! help: consider using a `let` binding to create a longer lived value
//!    |
//! 20 + let binding = doc.as_mut();
//! 21 ~ let mut mapping = binding.as_mapping_mut().context("missing root mapping")?;
//!    |
//! ```
//!
//! We could follow the recommendation and assign it to a local variable,
//! however that wouldn't be possible if we were using combinators such as
//! [`Option::and_then`].
//!
//! <br>
//!
//! Another important aspect of working with documents is that we can address
//! values *globally* through [identifiers][Id]. This makes it easier to store
//! desired modifications before applying them.
//!
//! ```
//! use anyhow::Context;
//! use nondestructive::yaml;
//!
//! let mut doc = yaml::from_slice(
//!     r"
//!     - 10
//!     - 24
//!     - 30
//!     "
//! )?;
//!
//! let mut edits = Vec::new();
//!
//! // Build a collection of edits:
//! for value in doc.as_ref().as_sequence().context("missing sequence")? {
//!     let Some(n) = value.as_u32() else {
//!         continue;
//!     };
//!
//!     if n % 10 == 0 {
//!         edits.push((value.id(), n / 10));
//!     }
//! }
//!
//! // Apply stored edits:
//! for (id, new_number) in edits {
//!     doc.value_mut(id).set_u32(new_number);
//! }
//!
//! assert_eq!(
//!     doc.to_string(),
//!     r"
//!     - 1
//!     - 24
//!     - 3
//!     "
//! );
//! # Ok::<_, anyhow::Error>(())
//! ```
//!
//! <br>
//!
//! ## Examples
//!
//! This provides a broader view of the available API, and the difference
//! between accessors and mutators.
//!
//! <br>
//!
//! ```
//! use anyhow::Context;
//! use nondestructive::yaml;
//!
//! let mut doc = yaml::from_slice(
//!     r"
//!     name: Descartes
//!     country: Grece
//!     "
//! )?;
//!
//! let mapping = doc.as_ref().as_mapping().context("missing mapping")?;
//! let name = mapping.get("name").context("missing name")?;
//! assert_eq!(name.as_str(), Some("Descartes"));
//!
//! let mut mapping = doc.as_mut().into_mapping_mut().context("missing mapping")?;
//! let mut name = mapping.get_mut("name").context("missing name")?;
//! name.set_string("Plato");
//!
//! let mut country = mapping.get_mut("country").context("missing country")?;
//! country.set_string("Greece");
//!
//! assert_eq!(
//!     doc.to_string(),
//!     r"
//!     name: Plato
//!     country: Greece
//!     "
//! );
//! # Ok::<_, anyhow::Error>(())
//! ```
//!
//! [`Option::and_then`]: https://doc.rust-lang.org/std/option/enum.Option.html#method.and_then
//! [`ValueMut::as_mapping_mut`]: https://docs.rs/nondestructive/latest/nondestructive/yaml/struct.ValueMut.html#method.as_mapping_mut
//! [`ValueMut::into_mapping_mut`]: https://docs.rs/nondestructive/latest/nondestructive/yaml/struct.ValueMut.html#method.into_mapping_mut
//! [`ValueMut`]: https://docs.rs/nondestructive/latest/nondestructive/yaml/struct.ValueMut.html
//! [Id]: https://docs.rs/nondestructive/latest/nondestructive/yaml/struct.Id.html
//! [yaml]: https://docs.rs/nondestructive/latest/nondestructive/yaml/index.html

#![deny(missing_docs)]
#![allow(clippy::module_inception)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::new_without_default)]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(all(feature = "alloc", feature = "std"))]
#[cfg_attr(doc_cfg, doc(cfg(all(feature = "alloc", feature = "std"))))]
pub mod yaml;
