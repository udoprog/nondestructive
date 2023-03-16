/// A generation counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct Generation(u64);

/// The index inside of the tree.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct Index(usize);

/// A pointer inside of the tree, includes the generation of the pointed to data.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Pointer {
    generation: Generation,
    index: Index,
}

/// An entry stored in the tree, which involves the data being stored and its
/// generation.
#[derive(Clone)]
pub(crate) struct Entry<T> {
    generation: Generation,
    data: T,
}

/// A tree capable of storing data.
#[derive(Clone)]
pub(crate) struct Slab<T> {
    generation: u64,
    data: slab::Slab<Entry<T>>,
}

impl<T> Slab<T> {
    /// Insert a value into the tree and return its pointer.
    pub(crate) fn insert(&mut self, data: T) -> Pointer {
        let generation = Generation(self.generation);
        self.generation = self.generation.wrapping_add(1);

        let index = self.data.insert(Entry { generation, data });

        Pointer {
            generation,
            index: Index(index),
        }
    }

    /// Get a value from the tree.
    pub(crate) fn get(&self, pointer: &Pointer) -> Option<&T> {
        let Pointer {
            generation,
            index: Index(index),
        } = *pointer;
        let entry = self.data.get(index)?;

        if entry.generation != generation {
            return None;
        }

        Some(&entry.data)
    }

    /// Get a value mutably from the tree.
    pub(crate) fn get_mut(&mut self, pointer: &Pointer) -> Option<&mut T> {
        let Pointer {
            generation,
            index: Index(index),
        } = *pointer;
        let entry = self.data.get_mut(index)?;

        if entry.generation != generation {
            return None;
        }

        Some(&mut entry.data)
    }

    /// Get the next pointer that will be inserted.
    pub(crate) fn pointer(&self) -> Pointer {
        Pointer {
            generation: Generation(self.generation),
            index: Index(self.data.vacant_key()),
        }
    }
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Self {
            generation: u64::default(),
            data: slab::Slab::default(),
        }
    }
}
