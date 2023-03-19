use core::slice;

use bstr::BStr;

use crate::yaml::data::{Data, ValueId};
use crate::yaml::Value;

/// An immutable iterator over a [`Mapping`][crate::yaml::mapping::Mapping].
///
/// See [`Mapping::iter`][crate::yaml::mapping::Mapping::iter].
pub struct Iter<'a> {
    data: &'a Data,
    iter: slice::Iter<'a, ValueId>,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(crate) fn new(data: &'a Data, slice: &'a [ValueId]) -> Self {
        Self {
            data,
            iter: slice.iter(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a BStr, Value<'a>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.data.mapping_item(*self.iter.next()?);
        let key = self.data.str(item.key.string);
        let value = Value::new(self.data, item.value);
        Some((key, value))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let item = self.data.mapping_item(*self.iter.nth(n)?);
        let key = self.data.str(item.key.string);
        let value = Value::new(self.data, item.value);
        Some((key, value))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Iter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let item = self.data.mapping_item(*self.iter.next_back()?);
        let key = self.data.str(item.key.string);
        let value = Value::new(self.data, item.value);
        Some((key, value))
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let item = self.data.mapping_item(*self.iter.nth(n)?);
        let key = self.data.str(item.key.string);
        let value = Value::new(self.data, item.value);
        Some((key, value))
    }
}

impl ExactSizeIterator for Iter<'_> {}
