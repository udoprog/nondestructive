use core::slice;

use crate::strings::Strings;
use crate::yaml::raw::RawListItem;
use crate::yaml::Value;

/// An immutable iterator over a [`List`][crate::yaml::list::List].
///
/// See [`List::iter`][crate::yaml::list::List::iter].
pub struct Iter<'a> {
    strings: &'a Strings,
    iter: slice::Iter<'a, RawListItem>,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(crate) fn new(strings: &'a Strings, slice: &'a [RawListItem]) -> Self {
        Self {
            strings,
            iter: slice.iter(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Value<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        Some(Value::new(self.strings, &item.value))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let item = self.iter.nth(n)?;
        Some(Value::new(self.strings, &item.value))
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
        Some(Value::new(self.strings, &item.value))
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let item = self.iter.nth(n)?;
        Some(Value::new(self.strings, &item.value))
    }
}

impl ExactSizeIterator for Iter<'_> {}
