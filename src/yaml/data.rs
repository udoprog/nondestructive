use std::collections::hash_map::{self, HashMap};
use std::fmt;
use std::hash::Hash;
use std::mem;
use std::num::NonZeroUsize;

use bstr::BStr;
use twox_hash::xxh3::{Hash128, HasherExt};

use crate::yaml::raw::{Layout, Raw, RawKind, RawList, RawTable};

/// The unique hash of a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(crate) struct StringId(u128);

impl fmt::Display for StringId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// An opaque identifier for a value inside of a [`Document`].
///
/// Is constructed through [`Value::id`], [`Table::id`], or [`List::id`] and can
/// be converted into a [`Value`] again through [`Document::value`] or
/// [`Document::value_mut`].
///
/// [`Value::id`]: crate::yaml::Value::id
/// [`Table::id`]: crate::yaml::Table::id
/// [`List::id`]: crate::yaml::List::id
/// [`Value`]: crate::yaml::Value
/// [`Document`]: crate::yaml::Document
/// [`Document::value`]: crate::yaml::Document::value
/// [`Document::value_mut`]: crate::yaml::Document::value_mut
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ValueId(NonZeroUsize);

impl ValueId {
    #[inline]
    fn get(&self) -> usize {
        self.0.get().wrapping_sub(1)
    }
}

impl fmt::Display for ValueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.get())
    }
}

/// Strings cache.
#[derive(Clone, Default)]
pub(crate) struct Data {
    strings: HashMap<StringId, Box<[u8]>>,
    slab: slab::Slab<Raw>,
}

impl Data {
    /// Get a string.
    #[inline]
    pub(crate) fn str(&self, id: &StringId) -> &BStr {
        let Some(string) = self.strings.get(id) else {
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
    pub(crate) fn layout(&self, index: ValueId) -> &Layout {
        if let Some(raw) = self.slab.get(index.get()) {
            return &raw.layout;
        }

        panic!("expected raw at {index}")
    }

    #[inline]
    pub(crate) fn raw(&self, index: ValueId) -> &Raw {
        if let Some(raw) = self.slab.get(index.get()) {
            return raw;
        }

        panic!("expected raw at {index}")
    }

    #[inline]
    pub(crate) fn raw_mut(&mut self, index: ValueId) -> &mut Raw {
        if let Some(raw) = self.slab.get_mut(index.get()) {
            return raw;
        }

        panic!("expected raw at {index}")
    }

    #[inline]
    pub(crate) fn list(&self, index: ValueId) -> &RawList {
        if let Some(Raw {
            kind: RawKind::List(raw),
            ..
        }) = self.slab.get(index.get())
        {
            return raw;
        }

        panic!("expected list at {index}")
    }

    #[inline]
    pub(crate) fn list_mut(&mut self, index: ValueId) -> &mut RawList {
        if let Some(Raw {
            kind: RawKind::List(raw),
            ..
        }) = self.slab.get_mut(index.get())
        {
            return raw;
        }

        panic!("expected list at {index}")
    }

    #[inline]
    pub(crate) fn table(&self, index: ValueId) -> &RawTable {
        if let Some(Raw {
            kind: RawKind::Table(raw),
            ..
        }) = self.slab.get(index.get())
        {
            return raw;
        }

        panic!("expected table at {index}")
    }

    #[inline]
    pub(crate) fn table_mut(&mut self, index: ValueId) -> &mut RawTable {
        if let Some(Raw {
            kind: RawKind::Table(raw),
            ..
        }) = self.slab.get_mut(index.get())
        {
            return raw;
        }

        panic!("expected table at {index}")
    }

    /// Insert a raw value and return its identifier.
    #[inline]
    pub(crate) fn insert_raw(&mut self, raw: Raw) -> ValueId {
        let index = self.slab.insert(raw);
        let index = NonZeroUsize::new(index.wrapping_add(1)).expect("ran out of ids");
        ValueId(index)
    }

    /// Drop a value recursively.
    #[inline]
    pub(crate) fn drop(&mut self, id: ValueId) {
        let Some(value) = self.slab.try_remove(id.get()) else {
            return;
        };

        self.drop_kind(value.kind);
    }

    /// Drop a raw value recursively.
    #[inline]
    pub(crate) fn drop_kind(&mut self, kind: RawKind) {
        match kind {
            RawKind::Table(raw) => {
                for item in raw.items {
                    let item = self.slab.remove(item.value.get());
                    self.drop_kind(item.kind);
                }
            }
            RawKind::List(raw) => {
                for item in raw.items {
                    let item = self.slab.remove(item.value.get());
                    self.drop_kind(item.kind);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn replace_raw(&mut self, index: ValueId, kind: RawKind) {
        let Some(value) = self.slab.get_mut(index.get()) else {
            return;
        };

        let removed = mem::replace(&mut value.kind, kind);
        self.drop_kind(removed);
    }
}
