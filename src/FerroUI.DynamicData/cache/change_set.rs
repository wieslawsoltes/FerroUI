//! `ChangeSet<TObject, TKey>` and `IChangeSet<TObject, TKey>` (`Cache/ChangeSet.cs`,
//! `Cache/IChangeSet.cs`, `IChangeSet.cs`).

use super::{Change, ChangeReason};
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// The changes of one edit of a cache (`ChangeSet<TObject, TKey>`): a shared, immutable list,
/// which every observer of a sequence receives as it is (docs/porting/reactive.md, section 5.4).
pub struct ChangeSet<T, K> {
    changes: Rc<Vec<Change<T, K>>>,
}

impl<T, K> Clone for ChangeSet<T, K> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone() }
    }
}

impl<T, K> Default for ChangeSet<T, K> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T, K> ChangeSet<T, K> {
    /// A change set without changes (`ChangeSet<TObject, TKey>.Empty`).
    pub fn empty() -> Self {
        Self { changes: Rc::new(Vec::new()) }
    }

    /// The changes, in order.
    pub fn changes(&self) -> &[Change<T, K>] {
        &self.changes
    }

    /// The number of changes (`Count`).
    pub fn count(&self) -> usize {
        self.changes.len()
    }

    fn count_of(&self, reason: ChangeReason) -> usize {
        self.changes.iter().filter(|change| change.reason() == reason).count()
    }

    /// The number of additions (`Adds`).
    pub fn adds(&self) -> usize {
        self.count_of(ChangeReason::Add)
    }

    /// The number of updates (`Updates`).
    pub fn updates(&self) -> usize {
        self.count_of(ChangeReason::Update)
    }

    /// The number of removals (`Removes`).
    pub fn removes(&self) -> usize {
        self.count_of(ChangeReason::Remove)
    }

    /// The number of refreshes (`Refreshes`).
    pub fn refreshes(&self) -> usize {
        self.count_of(ChangeReason::Refresh)
    }

    /// The number of moves (`Moves`).
    pub fn moves(&self) -> usize {
        self.count_of(ChangeReason::Moved)
    }

    /// The changes as a list of their own, to build on.
    pub fn to_vec(&self) -> Vec<Change<T, K>>
    where
        T: Clone,
        K: Clone,
    {
        self.changes.as_ref().clone()
    }

    /// Whether `self` and `other` are the same change set (`ReferenceEquals`).
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.changes, &other.changes)
    }
}

impl<T, K> From<Vec<Change<T, K>>> for ChangeSet<T, K> {
    /// A change set of the changes (`new ChangeSet<TObject, TKey>(IEnumerable<Change>)`).
    fn from(changes: Vec<Change<T, K>>) -> Self {
        Self { changes: Rc::new(changes) }
    }
}

impl<T, K> FromIterator<Change<T, K>> for ChangeSet<T, K> {
    fn from_iter<I: IntoIterator<Item = Change<T, K>>>(iter: I) -> Self {
        Self::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl<T, K> Deref for ChangeSet<T, K> {
    type Target = [Change<T, K>];

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T, K> IntoIterator for &'a ChangeSet<T, K> {
    type Item = &'a Change<T, K>;
    type IntoIter = std::slice::Iter<'a, Change<T, K>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.iter()
    }
}

impl<T: PartialEq, K: PartialEq> PartialEq for ChangeSet<T, K> {
    fn eq(&self, other: &Self) -> bool {
        self.changes == other.changes
    }
}

impl<T: fmt::Debug, K: fmt::Debug> fmt::Debug for ChangeSet<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.changes.iter()).finish()
    }
}

/// A change set of a cache, of any form (`IChangeSet<TObject, TKey>`): the plain one, and the
/// sorted, paged, virtual and contextual ones, which hold a plain one with more parts. The
/// operators of caches take any of them, as upstream's take its interface
/// (docs/porting/reactive.md, section 5.4).
pub trait IChangeSet<T, K>: Clone + 'static {
    /// The changes.
    fn change_set(&self) -> &ChangeSet<T, K>;
}

impl<T: 'static, K: 'static> IChangeSet<T, K> for ChangeSet<T, K> {
    fn change_set(&self) -> &ChangeSet<T, K> {
        self
    }
}
