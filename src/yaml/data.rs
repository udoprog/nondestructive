use core::fmt;
use core::hash::Hash;
use std::collections::hash_map::{self, HashMap};

use bstr::BStr;
use twox_hash::xxh3::{Hash128, HasherExt};

/// The unique hash of a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(crate) struct StringId(u128);

impl fmt::Display for StringId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Strings cache.
#[derive(Clone, Default)]
pub(crate) struct Data {
    strings: HashMap<StringId, Box<[u8]>>,
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
}
