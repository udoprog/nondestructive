//! A [`List`] of values.
//!
//! # Examples
//!
//! ```
//! use nondestructive::yaml;
//!
//! let doc = yaml::from_bytes(
//!     r#"
//!     - one
//!     - two
//!     - three
//!     "#,
//! )?;
//!
//! let root = doc.root().as_list().ok_or("missing root list")?;
//!
//! assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
//! assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
//! assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```

mod iter;
pub use self::iter::Iter;

mod list;
pub use self::list::List;

mod list_mut;
pub use self::list_mut::ListMut;
