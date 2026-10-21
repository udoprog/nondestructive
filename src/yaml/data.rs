use core::fmt;
use core::hash::Hash;
use core::mem;
use core::num::NonZeroUsize;

use alloc::boxed::Box;

use std::collections::hash_map::{self, HashMap};

use bstr::BStr;
#[cfg(feature = "serde-edits")]
use serde::{Deserialize, Serialize};
use twox_hash::xxh3::{Hash128, HasherExt};

use crate::yaml::raw;

/// The unique hash of a string.
///
/// Serializes as a lowercase hex string in human-readable formats, so that it
/// can be used as a map key in formats like JSON, and as bytes otherwise.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(crate) struct StringId([u8; 16]);

#[cfg(feature = "serde-edits")]
impl Serialize for StringId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.collect_str(&Hex(&self.0))
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

#[cfg(feature = "serde-edits")]
impl<'de> Deserialize<'de> for StringId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, SeqAccess, Unexpected, Visitor};

        struct StringIdVisitor;

        impl<'de> Visitor<'de> for StringIdVisitor {
            type Value = StringId;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a string id as 32 hex digits or 16 bytes")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                fn digit(b: u8) -> Option<u8> {
                    match b {
                        b'0'..=b'9' => Some(b - b'0'),
                        b'a'..=b'f' => Some(b - b'a' + 10),
                        b'A'..=b'F' => Some(b - b'A' + 10),
                        _ => None,
                    }
                }

                let invalid = || E::invalid_value(Unexpected::Str(v), &self);

                if v.len() != 32 {
                    return Err(invalid());
                }

                let mut out = [0u8; 16];

                for (o, pair) in out.iter_mut().zip(v.as_bytes().chunks_exact(2)) {
                    let (Some(hi), Some(lo)) = (digit(pair[0]), digit(pair[1])) else {
                        return Err(invalid());
                    };

                    *o = (hi << 4) | lo;
                }

                Ok(StringId(out))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match <[u8; 16]>::try_from(v) {
                    Ok(bytes) => Ok(StringId(bytes)),
                    Err(..) => Err(E::invalid_length(v.len(), &self)),
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut out = [0u8; 16];

                for (n, o) in out.iter_mut().enumerate() {
                    *o = seq
                        .next_element()?
                        .ok_or_else(|| A::Error::invalid_length(n, &self))?;
                }

                if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(A::Error::invalid_length(17, &self));
                }

                Ok(StringId(out))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(StringIdVisitor)
        } else {
            deserializer.deserialize_bytes(StringIdVisitor)
        }
    }
}

#[cfg(feature = "musli02")]
impl<M> musli_core02::Encode<M> for StringId {
    type Encode = Self;

    const IS_BITWISE_ENCODE: bool = false;

    #[inline]
    fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: musli_core02::Encoder<Mode = M>,
    {
        encoder.encode_array(&self.0)
    }

    #[inline]
    fn as_encode(&self) -> &Self::Encode {
        self
    }
}

#[cfg(feature = "musli02")]
impl<'de, M, A> musli_core02::Decode<'de, M, A> for StringId
where
    A: musli_core02::Allocator,
{
    const IS_BITWISE_DECODE: bool = false;

    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: musli_core02::Decoder<'de, Mode = M, Allocator = A>,
    {
        Ok(StringId(decoder.decode_array()?))
    }
}

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
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02, transparent)
)]
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
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
pub(crate) struct Entry {
    raw: raw::Raw,
    layout: raw::Layout,
}

/// Strings cache.
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde-edits", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "musli02",
    derive(musli_core02::Encode, musli_core02::Decode),
    musli(crate = musli_core02)
)]
pub(crate) struct Data {
    #[cfg_attr(feature = "musli02", musli(with = musli02::strings))]
    strings: HashMap<StringId, Box<[u8]>>,
    #[cfg_attr(feature = "musli02", musli(with = musli02::entries))]
    slab: slab::Slab<Entry>,
    /// Whitespace and comments after the root value, empty if unset.
    suffix: Option<StringId>,
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

    /// The text following the root value.
    #[inline]
    pub(crate) fn suffix(&self) -> &BStr {
        match self.suffix {
            Some(id) => self.str(id),
            None => BStr::new(""),
        }
    }

    /// Set the text following the root value.
    #[inline]
    pub(crate) fn set_suffix(&mut self, suffix: StringId) {
        self.suffix = Some(suffix);
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

    /// Set the prefix of a value.
    pub(crate) fn set_prefix(&mut self, id: Id, prefix: StringId) {
        if let Some(value) = self.slab.get_mut(id.get()) {
            value.layout.prefix = prefix;
        }
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

/// Field codecs for [`Data`] which musli-core has no impls for.
#[cfg(feature = "musli02")]
mod musli02 {
    /// Strings as a map from id to bytes, rather than to a sequence of `u8`.
    pub(super) mod strings {
        use alloc::boxed::Box;

        use std::collections::HashMap;

        use musli_core02::de::{DecodeBytes, EntryDecoder, MapDecoder};
        use musli_core02::en::{EntryEncoder, MapEncoder};
        use musli_core02::{Allocator, Decoder, Encoder};

        use crate::yaml::data::StringId;

        pub(crate) fn encode<E>(
            strings: &HashMap<StringId, Box<[u8]>>,
            encoder: E,
        ) -> Result<(), E::Error>
        where
            E: Encoder,
        {
            encoder.encode_map_fn(strings.len(), |map| {
                for (id, string) in strings {
                    map.encode_entry_fn(|entry| {
                        entry.encode_key()?.encode(id)?;
                        entry.encode_value()?.encode_bytes(string)?;
                        Ok(())
                    })?;
                }

                Ok(())
            })
        }

        pub(crate) fn decode<'de, D, A>(
            decoder: D,
        ) -> Result<HashMap<StringId, Box<[u8]>>, D::Error>
        where
            D: Decoder<'de, Allocator = A>,
            A: Allocator,
        {
            decoder.decode_map(|map| {
                let mut strings = HashMap::new();

                while let Some(mut entry) = map.decode_entry()? {
                    let id = entry.decode_key()?.decode()?;
                    let string = <Box<[u8]> as DecodeBytes<'de, D::Mode, A>>::decode_bytes(
                        entry.decode_value()?,
                    )?;
                    strings.insert(id, string);
                }

                Ok(strings)
            })
        }
    }

    /// The slab as a map from index to entry, like slab's serde impls.
    pub(super) mod entries {
        use alloc::vec::Vec;

        use musli_core02::de::{EntryDecoder, MapDecoder};
        use musli_core02::en::{EntryEncoder, MapEncoder};
        use musli_core02::{Allocator, Decode, Decoder, Encode, Encoder};

        use crate::yaml::data::Entry;

        pub(crate) fn encode<E, M>(slab: &slab::Slab<Entry>, encoder: E) -> Result<(), E::Error>
        where
            E: Encoder<Mode = M>,
            Entry: Encode<M>,
        {
            encoder.encode_map_fn(slab.len(), |map| {
                for (index, entry) in slab {
                    map.encode_entry_fn(|e| {
                        e.encode_key()?.encode(index)?;
                        e.encode_value()?.encode(entry)?;
                        Ok(())
                    })?;
                }

                Ok(())
            })
        }

        pub(crate) fn decode<'de, D, M, A>(decoder: D) -> Result<slab::Slab<Entry>, D::Error>
        where
            D: Decoder<'de, Mode = M, Allocator = A>,
            A: Allocator,
            Entry: Decode<'de, M, A>,
        {
            decoder.decode_map(|map| {
                let mut entries = Vec::new();

                while let Some(mut e) = map.decode_entry()? {
                    let index: usize = e.decode_key()?.decode()?;
                    let entry: Entry = e.decode_value()?.decode()?;
                    entries.push((index, entry));
                }

                Ok(entries.into_iter().collect())
            })
        }
    }
}
