//! `ItemWithIndex<T>` (`Kernel/ItemWithIndex.cs`).

use std::fmt;
use std::hash::{Hash, Hasher};

/// An item and its index (`ItemWithIndex<T>`): equal and hashed by the item alone, as upstream's.
#[derive(Clone, Copy, Debug)]
pub struct ItemWithIndex<T> {
    item: T,
    index: usize,
}

impl<T> ItemWithIndex<T> {
    /// The pair (`ItemWithIndex(T, int)`).
    pub fn new(item: T, index: usize) -> Self {
        Self { item, index }
    }

    /// The item (`Item`).
    pub fn item(&self) -> &T {
        &self.item
    }

    /// The index (`Index`).
    pub fn index(&self) -> usize {
        self.index
    }

    /// The item and the index.
    pub fn into_parts(self) -> (T, usize) {
        (self.item, self.index)
    }
}

impl<T: PartialEq> PartialEq for ItemWithIndex<T> {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}

impl<T: Eq> Eq for ItemWithIndex<T> {}

impl<T: Hash> Hash for ItemWithIndex<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.item.hash(state);
    }
}

impl<T: fmt::Display> fmt::Display for ItemWithIndex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.item, self.index)
    }
}
