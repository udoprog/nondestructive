use core::slice;

use crate::yaml::data::Data;
use crate::yaml::raw::RawSequenceItem;
use crate::yaml::Value;

/// An immumapping iterator over a [`Sequence`][crate::yaml::sequence::Sequence].
///
/// See [`Sequence::iter`][crate::yaml::sequence::Sequence::iter].
pub struct Iter<'a> {
    data: &'a Data,
    iter: slice::Iter<'a, RawSequenceItem>,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(crate) fn new(data: &'a Data, slice: &'a [RawSequenceItem]) -> Self {
        Self {
            data,
            iter: slice.iter(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Value<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        Some(Value::new(self.data, item.value))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let item = self.iter.nth(n)?;
        Some(Value::new(self.data, item.value))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Iter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let item = self.iter.next_back()?;
        Some(Value::new(self.data, item.value))
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let item = self.iter.nth(n)?;
        Some(Value::new(self.data, item.value))
    }
}

impl ExactSizeIterator for Iter<'_> {}
