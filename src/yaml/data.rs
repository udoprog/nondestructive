use std::collections::hash_map::{self, HashMap};
use std::fmt;
use std::hash::Hash;
use std::mem;
use std::num::NonZeroUsize;

use bstr::BStr;
use twox_hash::xxh3::{Hash128, HasherExt};

use crate::yaml::raw;

/// The unique hash of a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
/// [`Value::id`]: crate::yaml::Value::id
/// [`Mapping::id`]: crate::yaml::Mapping::id
/// [`Sequence::id`]: crate::yaml::Sequence::id
/// [`Value`]: crate::yaml::Value
/// [`Document`]: crate::yaml::Document
/// [`Document::value`]: crate::yaml::Document::value
/// [`Document::value_mut`]: crate::yaml::Document::value_mut
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ValueId(NonZeroUsize);

impl ValueId {
    #[inline]
    fn get(self) -> usize {
        self.0.get().wrapping_sub(1)
    }
}

impl fmt::Display for ValueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.get())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    raw: raw::Raw,
    layout: raw::Layout,
}

/// Strings cache.
#[derive(Clone, Default)]
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
    pub(crate) fn layout(&self, id: ValueId) -> &raw::Layout {
        if let Some(raw) = self.slab.get(id.get()) {
            return &raw.layout;
        }

        panic!("expected raw at {id}")
    }

    #[inline]
    pub(crate) fn prefix(&self, id: ValueId) -> &BStr {
        self.str(self.layout(id).prefix)
    }

    #[inline]
    pub(crate) fn raw(&self, id: ValueId) -> &raw::Raw {
        if let Some(raw) = self.slab.get(id.get()) {
            return &raw.raw;
        }

        panic!("expected raw at {id}")
    }

    #[inline]
    pub(crate) fn raw_mut(&mut self, id: ValueId) -> &mut raw::Raw {
        if let Some(raw) = self.slab.get_mut(id.get()) {
            return &mut raw.raw;
        }

        panic!("expected raw at {id}")
    }

    #[inline]
    pub(crate) fn sequence(&self, id: ValueId) -> &raw::Sequence {
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
    pub(crate) fn sequence_mut(&mut self, id: ValueId) -> &mut raw::Sequence {
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
    pub(crate) fn mapping(&self, id: ValueId) -> &raw::Mapping {
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
    pub(crate) fn sequence_item(&self, id: ValueId) -> &raw::SequenceItem {
        if let Some(Entry {
            raw: raw::Raw::SequenceItem(raw),
            ..
        }) = self.slab.get(id.get())
        {
            return raw;
        }

        panic!("expected mapping at {id}")
    }

    #[inline]
    pub(crate) fn mapping_item(&self, id: ValueId) -> &raw::MappingItem {
        if let Some(Entry {
            raw: raw::Raw::MappingItem(raw),
            ..
        }) = self.slab.get(id.get())
        {
            return raw;
        }

        panic!("expected mapping at {id}")
    }

    #[inline]
    pub(crate) fn mapping_mut(&mut self, id: ValueId) -> &mut raw::Mapping {
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
    pub(crate) fn insert(
        &mut self,
        raw: raw::Raw,
        prefix: StringId,
        parent: Option<ValueId>,
    ) -> ValueId {
        let index = self.slab.insert(Entry {
            raw,
            layout: raw::Layout { prefix, parent },
        });
        let index = NonZeroUsize::new(index.wrapping_add(1)).expect("ran out of ids");
        ValueId(index)
    }

    /// Drop a value recursively.
    #[inline]
    pub(crate) fn drop(&mut self, id: ValueId) {
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
    pub(crate) fn replace<T>(&mut self, id: ValueId, raw: T)
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
    pub(crate) fn replace_with(&mut self, id: ValueId, raw: raw::Raw, prefix: StringId) {
        let Some(value) = self.slab.get_mut(id.get()) else {
            return;
        };

        value.layout.prefix = prefix;
        let removed = mem::replace(&mut value.raw, raw);
        self.drop_kind(removed);
    }
}
