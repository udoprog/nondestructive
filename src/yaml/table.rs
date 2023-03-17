//! A [`Table`] of values.
//!
//! # Examples
//!
//! ```
//! use nondestructive::yaml;
//!
//! let doc = yaml::from_bytes(r#"
//! number1: 10
//! number2: 20
//! table:
//!   inner: 400
//! string3: "I am a quoted string!"
//! "#)?;
//!
//! let root = doc.root().as_table().ok_or("missing root table")?;
//!
//! assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
//! assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
//!
//! let table = root.get("table").and_then(|v| v.as_table()).ok_or("missing inner table")?;
//! assert_eq!(table.get("inner").and_then(|v| v.as_u32()), Some(400));
//!
//! assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```

mod iter;
pub use self::iter::Iter;

mod table;
pub use self::table::Table;

mod table_mut;
pub use self::table_mut::TableMut;
