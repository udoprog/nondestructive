//! A [`Mapping`] of values.
//!
//! # Examples
//!
//! ```
//! use anyhow::Context;
//! use nondestructive::yaml;
//!
//! let doc = yaml::from_slice(r#"
//! number1: 10
//! number2: 20
//! mapping:
//!   inner: 400
//! string3: "I am a quoted string!"
//! "#)?;
//!
//! let root = doc.root().as_mapping().ok_or("missing root mapping")?;
//!
//! assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
//! assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
//!
//! let mapping = root.get("mapping").and_then(|v| v.as_mapping()).ok_or("missing inner mapping")?;
//! assert_eq!(mapping.get("inner").and_then(|v| v.as_u32()), Some(400));
//!
//! assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```

mod iter;
pub use self::iter::Iter;

mod mapping;
pub use self::mapping::Mapping;

mod mapping_mut;
pub use self::mapping_mut::MappingMut;
