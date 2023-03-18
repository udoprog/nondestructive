//! [<img alt="github" src="https://img.shields.io/badge/github-udoprog/nondestructive-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/nondestructive)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/nondestructive.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/nondestructive)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-nondestructive-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/nondestructive)
//! [<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/udoprog/nondestructive/ci.yml?branch=main&style=for-the-badge" height="20">](https://github.com/udoprog/nondestructive/actions?query=branch%3Amain)
//!
//! Nondestructive editing over various file formats.
//!
//! **This project is as of yet, incomplete!**
//!
//! <br>
//!
//! ## Examples
//!
//! ```
//! use nondestructive::yaml;
//!
//! let mut doc = yaml::from_bytes(r#"
//!   number1: 10
//!   number2: 20
//!   table:
//!     inner: 400
//!   string3: "I am a quoted string!"
//! "#)?;
//!
//! let mut root = doc.root_mut();
//! let mut root = root.as_table_mut().ok_or("missing root table")?;
//! root.get_mut("number2").ok_or("missing inner table")?.set_u32(30);
//! root.get_mut("string3").ok_or("missing inner table")?.set_string("i-am-a-bare-string");
//!
//! assert_eq! {
//! doc.to_string(),
//! r#"
//!   number1: 10
//!   number2: 30
//!   table:
//!     inner: 400
//!   string3: i-am-a-bare-string
//! "#
//! };
//!
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```

#![deny(missing_docs)]
#![allow(clippy::module_inception)]
#![allow(clippy::module_name_repetitions)]

pub mod yaml;
