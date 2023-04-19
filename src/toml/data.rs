use std::collections::hash_map::{self, HashMap};
use std::fmt;
use std::hash::Hash;
use std::mem;
use std::num::NonZeroUsize;

use bstr::BStr;
#[cfg(feature = "serde-edits")]
use serde::{Deserialize, Serialize};
use twox_hash::xxh3::{Hash128, HasherExt};

use crate::toml::raw;

/// The unique hash of a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-edits", serde(transparent))]
#[repr(transparent)]
pub(crate) struct StringId(u128);

impl fmt::Display for StringId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// An opaque identifier for a value inside of a [`Document`].
///
/// Is constructed through [`Value::id`], [`Mapping::id`], or [`Sequence::id`] and can
/// be converted into a [`Value`] again through [`Document::value`] or
/// [`Document::value_mut`].
///
/// [`Value::id`]: crate::toml::Value::id
/// [`Mapping::id`]: crate::toml::Mapping::id
/// [`Sequence::id`]: crate::toml::Sequence::id
/// [`Value`]: crate::toml::Value
/// [`Document`]: crate::toml::Document
/// [`Document::value`]: crate::toml::Document::value
/// [`Document::value_mut`]: crate::toml::Document::value_mut
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
        let id = StringId(hasher.finish_ext());

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

        panic!("expected raw at {id}")
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
            raw::Raw::Table(table) => {
                for id in table.items {
                    self.drop(id);
                }
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
    pub(crate) fn replace_with(&mut self, id: Id, raw: raw::Raw, prefix: StringId) {
        let Some(value) = self.slab.get_mut(id.get()) else {
            return;
        };

        value.layout.prefix = prefix;
        let removed = mem::replace(&mut value.raw, raw);
        self.drop_kind(removed);
    }

    #[inline]
    pub(crate) fn table(&self, id: Id) -> &raw::Table {
        if let Some(Entry {
            raw: raw::Raw::Table(raw),
            ..
        }) = self.slab.get(id.get())
        {
            return raw;
        }

        panic!("expected table at {id}")
    }

    #[inline]
    pub(crate) fn table_item(&self, id: Id) -> &raw::TableItem {
        if let Some(Entry {
            raw: raw::Raw::TableItem(raw),
            ..
        }) = self.slab.get(id.get())
        {
            return raw;
        }

        panic!("expected table item at {id}")
    }
}
