//! `RangeChange<T>` (`List/RangeChange.cs`).

/// The items of a change of a range, and the index of the first (`RangeChange<T>`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RangeChange<T> {
    items: Vec<T>,
    index: Option<usize>,
}

impl<T> Default for RangeChange<T> {
    /// No items, no index (`RangeChange<T>.Empty`).
    fn default() -> Self {
        Self { items: Vec::new(), index: None }
    }
}

impl<T> RangeChange<T> {
    /// `RangeChange(IEnumerable<T>, int index = -1)`.
    pub fn new(items: Vec<T>, index: Option<usize>) -> Self {
        Self { items, index }
    }

    /// The number of items (`Count`).
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// The index of the first item (`Index`).
    pub fn index(&self) -> Option<usize> {
        self.index
    }

    /// The items.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    /// Adds an item at the end (`Add`).
    pub fn add(&mut self, item: T) {
        self.items.push(item);
    }

    /// Inserts an item (`Insert`).
    pub fn insert(&mut self, index: usize, item: T) {
        self.items.insert(index, item);
    }

    /// Sets the index of the first item (`SetStartingIndex`).
    pub fn set_starting_index(&mut self, index: Option<usize>) {
        self.index = index;
    }
}

impl<'a, T> IntoIterator for &'a RangeChange<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}
