//! `ISortedChangeSet<TObject, TKey>` and `SortedChangeSet<TObject, TKey>`
//! (`Cache/ISortedChangeSet.cs`, `Cache/SortedChangeSet.cs`).

use super::{ChangeSet, IChangeSet, KeyValueCollection};
use std::fmt;
use std::ops::Deref;

/// A change set with the sorted items it leads to (`ISortedChangeSet<TObject, TKey>`): the sorted
/// change set, and the paged and virtual ones, which are sorted too.
pub trait ISortedChangeSet<T, K>: IChangeSet<T, K> {
    /// The items, sorted (`SortedItems`).
    fn sorted_items(&self) -> &KeyValueCollection<T, K>;

    /// The change set as a plain sorted one (the upcast to `ISortedChangeSet<TObject, TKey>`).
    fn to_sorted_change_set(&self) -> SortedChangeSet<T, K> {
        SortedChangeSet::new(self.sorted_items().clone(), self.change_set().clone())
    }
}

/// The changes of a sorted cache with its items in order (`SortedChangeSet<TObject, TKey>`).
///
/// It dereferences to its [`ChangeSet`], as upstream's derives from it.
pub struct SortedChangeSet<T, K> {
    changes: ChangeSet<T, K>,
    sorted_items: KeyValueCollection<T, K>,
}

impl<T, K> Clone for SortedChangeSet<T, K> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone(), sorted_items: self.sorted_items.clone() }
    }
}

impl<T, K> Default for SortedChangeSet<T, K> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T, K> SortedChangeSet<T, K> {
    /// The changes `updates` that lead to `sorted_items` (`SortedChangeSet(IKeyValueCollection,
    /// IEnumerable<Change>)`).
    pub fn new(sorted_items: KeyValueCollection<T, K>, updates: ChangeSet<T, K>) -> Self {
        Self { changes: updates, sorted_items }
    }

    /// A change set without changes or items (`SortedChangeSet<TObject, TKey>.Empty`).
    pub fn empty() -> Self {
        Self::new(KeyValueCollection::empty(), ChangeSet::empty())
    }

    /// The items, sorted (`SortedItems`).
    pub fn sorted_items(&self) -> &KeyValueCollection<T, K> {
        &self.sorted_items
    }

    /// The changes.
    pub fn changes(&self) -> &ChangeSet<T, K> {
        &self.changes
    }
}

impl<T, K> Deref for SortedChangeSet<T, K> {
    type Target = ChangeSet<T, K>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T, K> IntoIterator for &'a SortedChangeSet<T, K> {
    type Item = &'a super::Change<T, K>;
    type IntoIter = std::slice::Iter<'a, super::Change<T, K>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.changes().iter()
    }
}

impl<T: 'static, K: 'static> IChangeSet<T, K> for SortedChangeSet<T, K> {
    fn change_set(&self) -> &ChangeSet<T, K> {
        &self.changes
    }
}

impl<T: 'static, K: 'static> ISortedChangeSet<T, K> for SortedChangeSet<T, K> {
    fn sorted_items(&self) -> &KeyValueCollection<T, K> {
        &self.sorted_items
    }

    fn to_sorted_change_set(&self) -> SortedChangeSet<T, K> {
        self.clone()
    }
}

impl<T: PartialEq, K: PartialEq> PartialEq for SortedChangeSet<T, K> {
    /// The same sorted items (`Equals`: `SortedItems.SequenceEqual(other.SortedItems)`).
    fn eq(&self, other: &Self) -> bool {
        self.sorted_items == other.sorted_items
    }
}

impl<T, K> fmt::Display for SortedChangeSet<T, K> {
    /// `ToString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SortedChangeSet. Count= {}. Updates = {}", self.sorted_items.count(), self.changes.count())
    }
}

impl<T: fmt::Debug, K: fmt::Debug> fmt::Debug for SortedChangeSet<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SortedChangeSet").field("changes", &self.changes).field("sorted_items", &self.sorted_items).finish()
    }
}
