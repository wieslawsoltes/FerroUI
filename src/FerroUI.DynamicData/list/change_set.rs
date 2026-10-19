//! `ChangeSet<T>` and `IChangeSet<T>` (`List/ChangeSet.cs`, `List/IChangeSet.cs`).

use super::{Change, ListChangeReason};
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// The changes of one edit of a list (`ChangeSet<T>`): shared and immutable, as the change sets of
/// caches are.
pub struct ChangeSet<T> {
    changes: Rc<Vec<Change<T>>>,
}

impl<T> Clone for ChangeSet<T> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone() }
    }
}

impl<T> Default for ChangeSet<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> ChangeSet<T> {
    /// A change set without changes (`ChangeSet<T>.Empty`).
    pub fn empty() -> Self {
        Self { changes: Rc::new(Vec::new()) }
    }

    /// The changes, in order.
    pub fn changes(&self) -> &[Change<T>] {
        &self.changes
    }

    /// The number of changes (`Count`).
    pub fn count(&self) -> usize {
        self.changes.len()
    }

    /// The number of items added, ranges counted by their items (`Adds`).
    pub fn adds(&self) -> usize {
        self.changes
            .iter()
            .map(|change| match change.reason() {
                ListChangeReason::Add => 1,
                ListChangeReason::AddRange => change.range().count(),
                _ => 0,
            })
            .sum()
    }

    /// The number of moves (`Moves`).
    pub fn moves(&self) -> usize {
        self.changes.iter().filter(|change| change.reason() == ListChangeReason::Moved).count()
    }

    /// The number of refreshes (`Refreshes`).
    pub fn refreshes(&self) -> usize {
        self.changes.iter().filter(|change| change.reason() == ListChangeReason::Refresh).count()
    }

    /// The number of items removed, ranges and clears counted by their items (`Removes`).
    pub fn removes(&self) -> usize {
        self.changes
            .iter()
            .map(|change| match change.reason() {
                ListChangeReason::Remove => 1,
                ListChangeReason::RemoveRange | ListChangeReason::Clear => change.range().count(),
                _ => 0,
            })
            .sum()
    }

    /// The number of replacements (`Replaced`).
    pub fn replaced(&self) -> usize {
        self.changes.iter().filter(|change| change.reason() == ListChangeReason::Replace).count()
    }

    /// The number of changed items (`TotalChanges`).
    pub fn total_changes(&self) -> usize {
        self.adds() + self.removes() + self.refreshes() + self.replaced() + self.moves()
    }

    /// Whether `self` and `other` are the same change set (`ReferenceEquals`).
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.changes, &other.changes)
    }
}

impl<T> From<Vec<Change<T>>> for ChangeSet<T> {
    fn from(changes: Vec<Change<T>>) -> Self {
        Self { changes: Rc::new(changes) }
    }
}

impl<T> FromIterator<Change<T>> for ChangeSet<T> {
    fn from_iter<I: IntoIterator<Item = Change<T>>>(iter: I) -> Self {
        Self::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl<T> Deref for ChangeSet<T> {
    type Target = [Change<T>];

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T> IntoIterator for &'a ChangeSet<T> {
    type Item = &'a Change<T>;
    type IntoIter = std::slice::Iter<'a, Change<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.iter()
    }
}

impl<T: PartialEq> PartialEq for ChangeSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.changes == other.changes
    }
}

impl<T: fmt::Debug> fmt::Debug for ChangeSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.changes.iter()).finish()
    }
}

/// A change set of a list, of any form (`IChangeSet<T>`): the plain one, and the paged and
/// virtual ones, which hold a plain one with more parts.
pub trait IChangeSet<T>: Clone + 'static {
    /// The changes.
    fn change_set(&self) -> &ChangeSet<T>;
}

impl<T: 'static> IChangeSet<T> for ChangeSet<T> {
    fn change_set(&self) -> &ChangeSet<T> {
        self
    }
}
