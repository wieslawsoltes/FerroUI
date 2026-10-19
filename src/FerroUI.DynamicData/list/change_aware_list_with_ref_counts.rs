//! `ChangeAwareListWithRefCounts<T>` (`List/ChangeAwareListWithRefCounts.cs`): a
//! [`ChangeAwareList`] that counts how many times it holds each item, which answers `Contains`
//! without a search (the combiners of lists).
#![allow(dead_code)]

use super::internal::ReferenceCountTracker;
use super::change_aware_list::ChangeAwareListHooks;
use super::ChangeAwareList;
use std::hash::Hash;

/// The overrides of `ChangeAwareListWithRefCounts<T>`, as hooks of the list.
struct RefCountHooks<T> {
    tracker: ReferenceCountTracker<T>,
}

impl<T: Eq + Hash + Clone> ChangeAwareListHooks<T> for RefCountHooks<T> {
    /// `InsertItem`.
    fn on_insert_item(&mut self, item: &T) {
        self.tracker.add(item);
    }

    /// `OnInsertItems`.
    fn on_insert_items(&mut self, items: &[T]) {
        for item in items {
            self.tracker.add(item);
        }
    }

    /// `RemoveItem`.
    fn on_remove_item(&mut self, item: &T) {
        self.tracker.remove(item);
    }

    /// `OnRemoveItems`.
    fn on_remove_items(&mut self, items: &[T]) {
        for item in items {
            self.tracker.remove(item);
        }
    }

    /// `OnSetItem`.
    fn on_set_item(&mut self, new_item: &T, old_item: &T) {
        self.tracker.remove(old_item);
        self.tracker.add(new_item);
    }

    /// `Clear`.
    fn on_clear(&mut self) {
        self.tracker.clear();
    }

    /// `Contains`: by the counts.
    fn contains(&self, item: &T) -> Option<bool> {
        Some(self.tracker.contains(item))
    }
}

/// An empty list that counts its items (`new ChangeAwareListWithRefCounts<T>()`).
pub(crate) fn change_aware_list_with_ref_counts<T: Eq + Hash + Clone + 'static>() -> ChangeAwareList<T> {
    ChangeAwareList::with_hooks(Box::new(RefCountHooks { tracker: ReferenceCountTracker::new() }))
}
