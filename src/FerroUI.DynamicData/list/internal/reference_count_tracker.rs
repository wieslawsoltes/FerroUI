//! `ReferenceCountTracker<T>` (`List/Internal/ReferenceCountTracker.cs`): how many times each item
//! is held.
#![allow(dead_code)]

use ferroui_system::collections::generic::Dictionary;
use std::hash::Hash;

/// The counts (`ReferenceCountTracker<T>`).
pub(crate) struct ReferenceCountTracker<T> {
    reference_counts: Dictionary<T, usize>,
}

impl<T: Eq + Hash + Clone> Default for ReferenceCountTracker<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Eq + Hash + Clone> ReferenceCountTracker<T> {
    pub(crate) fn new() -> Self {
        Self { reference_counts: Dictionary::new() }
    }

    /// The items held (`Items`).
    pub(crate) fn items(&self) -> Vec<T> {
        self.reference_counts.keys().cloned().collect()
    }

    /// The count of `item` (the indexer).
    ///
    /// # Panics
    /// When the item is not held (`KeyNotFoundException`).
    pub(crate) fn get(&self, item: &T) -> usize {
        *self.reference_counts.get(item).expect("The given key was not present in the dictionary.")
    }

    /// Counts one more hold of `item`; true when it is its first (`Add`).
    pub(crate) fn add(&mut self, item: &T) -> bool {
        match self.reference_counts.get_mut(item) {
            Some(count) => {
                *count += 1;
                false
            }
            None => {
                self.reference_counts.add(item.clone(), 1);
                true
            }
        }
    }

    /// Forgets every item (`Clear`).
    pub(crate) fn clear(&mut self) {
        self.reference_counts.clear();
    }

    /// Whether `item` is held (`Contains`).
    pub(crate) fn contains(&self, item: &T) -> bool {
        self.reference_counts.contains_key(item)
    }

    /// Counts one hold of `item` less; true when it was its last (`Remove`).
    ///
    /// # Panics
    /// When the item is not held (`KeyNotFoundException`).
    pub(crate) fn remove(&mut self, item: &T) -> bool {
        let current_count = self.get(item);
        if current_count == 1 {
            self.reference_counts.remove(item);
            return true;
        }
        *self.reference_counts.get_mut(item).expect("the item is held") = current_count - 1;
        false
    }
}
