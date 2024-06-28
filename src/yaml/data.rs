use std::collections::hash_map::{self, HashMap};
use std::fmt;
use std::hash::Hash;
use std::mem;
use std::num::NonZeroUsize;

use bstr::BStr;
#[cfg(feature = "serde-edits")]
use serde::{Deserialize, Serialize};
use twox_hash::xxh3::{Hash128, HasherExt};

use crate::yaml::raw;

/// The unique hash of a string.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(transparent))]
#[repr(transparent)]
pub(crate) struct StringId([u8; 16]);

impl fmt::Display for StringId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Hex(&self.0))
    }
}

impl fmt::Debug for StringId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StringId").field(&Hex(&self.0)).finish()
    }
}

struct Hex<'a>(&'a [u8]);

impl fmt::Display for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }

        Ok(())
    }
}

impl fmt::Debug for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// An opaque identifier for a value inside of a [`Document`].
///
/// Is constructed through [`Value::id`], [`Mapping::id`], or [`Sequence::id`] and can
/// be converted into a [`Value`] again through [`Document::value`] or
/// [`Document::value_mut`].
///
/// [`Value::id`]: crate::yaml::Value::id
/// [`Mapping::id`]: crate::yaml::Mapping::id
/// [`Sequence::id`]: crate::yaml::Sequence::id
/// [`Value`]: crate::yaml::Value
/// [`Document`]: crate::yaml::Document
/// [`Document::value`]: crate::yaml::Document::value
/// [`Document::value_mut`]: crate::yaml::Document::value_mut
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(transparent))]
#[repr(transparent)]
pub struct Id(NonZeroUsize);

impl Id {
    #[inline]
    fn get(self) -> usize {
        self.0.get().wrapping_sub(1)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.get())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct Entry {
    raw: raw::Raw,
    layout: raw::Layout,
}

/// Strings cache.
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
pub(crate) struct Data {
    strings: HashMap<StringId, Box<[u8]>>,
    slab: slab::Slab<Entry>,
}

impl Data {
    /// Get a string.
    #[inline]
    #[must_use]
    pub(crate) fn str(&self, id: StringId) -> &BStr {
        let Some(string) = self.strings.get(&id) else {
            panic!("missing string with id {id}");
        };

        BStr::new(string.as_ref())
    }

    /// Insert a string into the string cache.
    pub(crate) fn insert_str<B>(&mut self, string: B) -> StringId
    where
        B: AsRef<[u8]>,
    {
        let mut hasher = Hash128::default();
        string.as_ref().hash(&mut hasher);
        let hash = hasher.finish_ext();
        let hash = hash.to_le_bytes();
        let id = StringId(hash);

        if let hash_map::Entry::Vacant(e) = self.strings.entry(id) {
            e.insert(string.as_ref().into());
        }

        id
    }

    #[inline]
    pub(crate) fn layout(&self, id: Id) -> &raw::Layout {
        if let Some(raw) = self.slab.get(id.get()) {
            return &raw.layout;
        }

        panic!("expected layout at {id}")
    }

    #[inline]
    pub(crate) fn prefix(&self, id: Id) -> &BStr {
        self.str(self.layout(id).prefix)
    }

    #[inline]
    pub(crate) fn pair(&self, id: Id) -> (&raw::Raw, &raw::Layout) {
        if let Some(raw) = self.slab.get(id.get()) {
            return (&raw.raw, &raw.layout);
        }

        panic!("expected raw at {id}")
    }

    #[inline]
    pub(crate) fn raw(&self, id: Id) -> &raw::Raw {
        if let Some(raw) = self.slab.get(id.get()) {
            return &raw.raw;
        }

        panic!("expected raw at {id}")
    }

    #[inline]
    pub(crate) fn raw_mut(&mut self, id: Id) -> &mut raw::Raw {
        if let Some(raw) = self.slab.get_mut(id.get()) {
            return &mut raw.raw;
        }

        panic!("expected raw at {id}")
    }

    #[inline]
    pub(crate) fn sequence(&self, id: Id) -> &raw::Sequence {
        if let Some(Entry {
            raw: raw::Raw::Sequence(raw),
            ..
        }) = self.slab.get(id.get())
        {
            return raw;
        }

        panic!("expected sequence at {id}")
    }

    #[inline]
    pub(crate) fn sequence_mut(&mut self, id: Id) -> &mut raw::Sequence {
        if let Some(Entry {
            raw: raw::Raw::Sequence(raw),
            ..
        }) = self.slab.get_mut(id.get())
        {
            return raw;
        }

        panic!("expected sequence at {id}")
    }

    #[inline]
    pub(crate) fn mapping(&self, id: Id) -> &raw::Mapping {
        if let Some(Entry {
            raw: raw::Raw::Mapping(raw),
            ..
        }) = self.slab.get(id.get())
        {
            return raw;
        }

        panic!("expected mapping at {id}")
    }

    #[inline]
    pub(crate) fn sequence_item(&self, id: Id) -> &raw::SequenceItem {
        if let Some(Entry {
            raw: raw::Raw::SequenceItem(raw),
            ..
        }) = self.slab.get(id.get())
        {
            return raw;
        }

        panic!("expected sequence item at {id}")
    }

    #[inline]
    pub(crate) fn mapping_item(&self, id: Id) -> &raw::MappingItem {
        if let Some(Entry {
            raw: raw::Raw::MappingItem(raw),
            ..
        }) = self.slab.get(id.get())
        {
            return raw;
        }

        panic!("expected mapping item at {id}")
    }

    #[inline]
    pub(crate) fn mapping_mut(&mut self, id: Id) -> &mut raw::Mapping {
        if let Some(Entry {
            raw: raw::Raw::Mapping(raw),
            ..
        }) = self.slab.get_mut(id.get())
        {
            return raw;
        }

        panic!("expected mapping at {id}")
    }

    /// Insert a raw value and return its identifier.
    #[inline]
    pub(crate) fn insert(&mut self, raw: raw::Raw, prefix: StringId, parent: Option<Id>) -> Id {
        let index = self.slab.insert(Entry {
            raw,
            layout: raw::Layout { prefix, parent },
        });
        let index = NonZeroUsize::new(index.wrapping_add(1)).expect("ran out of ids");
        Id(index)
    }

    /// Drop a value recursively.
    #[inline]
    pub(crate) fn drop(&mut self, id: Id) {
        let Some(value) = self.slab.try_remove(id.get()) else {
            return;
        };

        self.drop_kind(value.raw);
    }

    /// Drop a raw value recursively.
    #[inline]
    pub(crate) fn drop_kind(&mut self, raw: raw::Raw) {
        match raw {
            raw::Raw::Mapping(raw) => {
                for item in raw.items {
                    self.drop(item);
                }
            }
            raw::Raw::MappingItem(raw) => {
                let item = self.slab.remove(raw.value.get());
                self.drop_kind(item.raw);
            }
            raw::Raw::Sequence(raw) => {
                for item in raw.items {
                    self.drop(item);
                }
            }
            raw::Raw::SequenceItem(raw) => {
                let item = self.slab.remove(raw.value.get());
                self.drop_kind(item.raw);
            }
            _ => {}
        }
    }

    /// Replace a raw value.
    pub(crate) fn replace<T>(&mut self, id: Id, raw: T)
    where
        T: Into<raw::Raw>,
    {
        let Some(value) = self.slab.get_mut(id.get()) else {
            return;
        };

        let removed = mem::replace(&mut value.raw, raw.into());
        self.drop_kind(removed);
    }

    /// Replace with indentation.
    pub(crate) fn replace_with(&mut self, id: Id, prefix: StringId, raw: raw::Raw) {
        let Some(value) = self.slab.get_mut(id.get()) else {
            return;
        };

        value.layout.prefix = prefix;
        let removed = mem::replace(&mut value.raw, raw);
        self.drop_kind(removed);
    }
}
