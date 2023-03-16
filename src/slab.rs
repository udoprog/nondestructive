/// The index inside of the tree.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct Pointer(usize);

/// A tree capable of storing data.
#[derive(Clone)]
pub(crate) struct Slab<T> {
    data: slab::Slab<T>,
}

impl<T> Slab<T> {
    /// Insert a value into the tree and return its pointer.
    pub(crate) fn insert(&mut self, data: T) -> Pointer {
        let index = self.data.insert(data);
        Pointer(index)
    }

    /// Get a value from the tree.
    pub(crate) fn get(&self, pointer: &Pointer) -> Option<&T> {
        let Pointer(index) = *pointer;
        self.data.get(index)
    }

    /// Test if slab contains the given pointer.
    pub(crate) fn contains<F>(&self, pointer: &Pointer, test: F) -> bool
    where
        F: FnOnce(&T) -> bool,
    {
        let Pointer(index) = *pointer;

        let Some(data) = self.data.get(index) else {
            return false;
        };

        test(data)
    }

    /// Get a value mutably from the tree.
    pub(crate) fn get_mut(&mut self, pointer: &Pointer) -> Option<&mut T> {
        let Pointer(index) = *pointer;
        self.data.get_mut(index)
    }
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Self {
            data: slab::Slab::default(),
        }
    }
}
