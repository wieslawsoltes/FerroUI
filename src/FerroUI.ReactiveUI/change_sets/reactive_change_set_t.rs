//! `ReactiveChangeSet<T>` (`ReactiveUI.Core/ChangeSets/ReactiveChangeSet{T}.cs`).

use super::{IReactiveChangeSet, IReactiveChangeSetOf, ReactiveChange, ReactiveChangeReason};
use std::fmt;
use std::ops::Index;
use std::rc::Rc;

/// A batch of changes of a collection (`ReactiveChangeSet<T>`), with its counts of additions and
/// removals.
///
/// The batch is shared and immutable: a clone is the same batch, as passing the reference of
/// upstream's class is, so a stream hands every observer the same batch.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the streams of change sets carry this type, where
/// upstream's carry the interface `IReactiveChangeSet<T>`, which this type implements
/// ([`IReactiveChangeSetOf`]).
pub struct ReactiveChangeSet<T> {
    changes: Rc<[ReactiveChange<T>]>,
    adds: usize,
    removes: usize,
}

impl<T> Clone for ReactiveChangeSet<T> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone(), adds: self.adds, removes: self.removes }
    }
}

impl<T: fmt::Debug> fmt::Debug for ReactiveChangeSet<T> {
    /// `[DebuggerDisplay("Count = {Count}, Adds = {Adds}, Removes = {Removes}")]`, with the changes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveChangeSet")
            .field("count", &self.changes.len())
            .field("adds", &self.adds)
            .field("removes", &self.removes)
            .field("changes", &&*self.changes)
            .finish()
    }
}

impl<T> ReactiveChangeSet<T> {
    /// A batch of `changes` (`ReactiveChangeSet(List<ReactiveChange<T>>)`).
    pub fn new(changes: Vec<ReactiveChange<T>>) -> Self {
        let mut adds = 0;
        let mut removes = 0;
        for change in &changes {
            match change.reason() {
                ReactiveChangeReason::Add => adds += 1,
                ReactiveChangeReason::Remove => removes += 1,
                _ => {}
            }
        }
        Self { changes: changes.into(), adds, removes }
    }

    /// The number of changes (`Count`).
    pub fn count(&self) -> usize {
        self.changes.len()
    }

    /// The number of additions (`Adds`).
    pub fn adds(&self) -> usize {
        self.adds
    }

    /// The number of removals (`Removes`).
    pub fn removes(&self) -> usize {
        self.removes
    }

    /// The change at `index`; `None` past the end.
    pub fn get(&self, index: usize) -> Option<&ReactiveChange<T>> {
        self.changes.get(index)
    }

    /// The changes, in order (`GetEnumerator`).
    pub fn iter(&self) -> std::slice::Iter<'_, ReactiveChange<T>> {
        self.changes.iter()
    }

    /// Whether `self` and `other` are the same batch.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.changes, &other.changes)
    }
}

impl<T> Index<usize> for ReactiveChangeSet<T> {
    type Output = ReactiveChange<T>;

    /// The change at `index` (`this[int]`).
    ///
    /// # Panics
    /// When `index` is out of range (`ArgumentOutOfRangeException`).
    fn index(&self, index: usize) -> &ReactiveChange<T> {
        &self.changes[index]
    }
}

impl<'a, T> IntoIterator for &'a ReactiveChangeSet<T> {
    type Item = &'a ReactiveChange<T>;
    type IntoIter = std::slice::Iter<'a, ReactiveChange<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.iter()
    }
}

impl<T> IReactiveChangeSet for ReactiveChangeSet<T> {
    fn adds(&self) -> usize {
        self.adds
    }

    fn removes(&self) -> usize {
        self.removes
    }
}

impl<T> IReactiveChangeSetOf<T> for ReactiveChangeSet<T> {
    fn count(&self) -> usize {
        self.changes.len()
    }

    fn get(&self, index: usize) -> Option<&ReactiveChange<T>> {
        self.changes.get(index)
    }

    fn changes(&self) -> &[ReactiveChange<T>] {
        &self.changes
    }
}
