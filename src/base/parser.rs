use std::{ops::Range, slice::SliceIndex};

#[derive(Clone)]
pub(crate) struct Parser<'a> {
    input: &'a [u8],
    n: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self { input, n: 0 }
    }

    /// Get the given index.
    #[inline]
    pub(crate) fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[u8]>,
    {
        self.input.get(index)
    }

    /// Get remaining span of the parser.
    #[inline]
    pub(crate) fn span(&self) -> Range<usize> {
        self.n..self.input.len()
    }

    /// Bump a single byte of input.
    #[inline]
    pub(crate) fn bump(&mut self, n: usize) {
        self.n = self.n.wrapping_add(n).min(self.input.len());
    }

    /// Get a string from the given starting position to current cursor
    /// location.
    #[inline]
    pub(crate) fn string(&self, start: usize) -> &'a [u8] {
        self.input.get(start..self.n).unwrap_or_default()
    }

    /// Test if eof.
    #[inline]
    pub(crate) fn is_eof(&self) -> bool {
        self.n == self.input.len()
    }

    /// Find the given character.
    pub(crate) fn find(&mut self, a: u8) {
        let input = self.input.get(self.n..).unwrap_or_default();

        if let Some(n) = memchr::memchr(a, input) {
            self.bump(n);
        } else {
            self.n = self.input.len();
        }
    }

    /// Find the given character.
    pub(crate) fn find2(&mut self, a: u8, b: u8) {
        let input = self.input.get(self.n..).unwrap_or_default();

        if let Some(n) = memchr::memchr2(a, b, input) {
            self.bump(n);
        } else {
            self.n = self.input.len();
        }
    }

    /// Peek the next value.
    #[inline]
    pub(crate) fn peek(&self) -> u8 {
        let Some(&b) = self.input.get(self.n) else {
            return 0;
        };

        b
    }

    /// Peek the next next value.
    #[inline]
    pub(crate) fn peek2(&self) -> (u8, u8) {
        let b0 = self.peek();

        let Some(&b) = self.input.get(self.n.wrapping_add(1)) else {
            return (b0, 0);
        };

        (b0, b)
    }

    /// Get the given position.
    #[inline]
    pub(crate) fn pos(&self) -> usize {
        self.n
    }
}
