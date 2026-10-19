//! `DistinctChangeSet<T>` and `IDistinctChangeSet<T>` (`Cache/DistinctChangeSet.cs`,
//! `Cache/IDistinctChangeSet.cs`), with the aggregator of their sequences
//! (`Cache/Tests/DistinctChangeSetAggregator.cs`).

use super::testing::ChangeSetAggregator;
use super::{Change, ChangeSet, IChangeSet};
use std::fmt;
use std::ops::Deref;

/// The changes of the distinct values of a sequence (`DistinctChangeSet<T>`,
/// `IDistinctChangeSet<T>`): each value is its own key.
pub struct DistinctChangeSet<T> {
    changes: ChangeSet<T, T>,
}

impl<T> Clone for DistinctChangeSet<T> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone() }
    }
}

impl<T> Default for DistinctChangeSet<T> {
    fn default() -> Self {
        Self { changes: ChangeSet::empty() }
    }
}

impl<T> DistinctChangeSet<T> {
    /// The change set of `changes` (`DistinctChangeSet(IEnumerable<Change<T, T>>)`).
    pub fn new(changes: Vec<Change<T, T>>) -> Self {
        Self { changes: ChangeSet::from(changes) }
    }

    /// The plain change set.
    pub fn into_change_set(self) -> ChangeSet<T, T> {
        self.changes
    }
}

impl<T> From<Vec<Change<T, T>>> for DistinctChangeSet<T> {
    fn from(changes: Vec<Change<T, T>>) -> Self {
        Self::new(changes)
    }
}

impl<T> Deref for DistinctChangeSet<T> {
    type Target = ChangeSet<T, T>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<T: 'static> IChangeSet<T, T> for DistinctChangeSet<T> {
    fn change_set(&self) -> &ChangeSet<T, T> {
        &self.changes
    }
}

impl<T: PartialEq> PartialEq for DistinctChangeSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.changes == other.changes
    }
}

impl<T: fmt::Debug> fmt::Debug for DistinctChangeSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.changes.fmt(f)
    }
}

/// Records what a sequence of distinct change sets produces (`DistinctChangeSetAggregator<TValue>`):
/// the aggregator of the plain change sets, which `as_aggregator` makes of the sequence.
pub type DistinctChangeSetAggregator<T> = ChangeSetAggregator<T, T>;
