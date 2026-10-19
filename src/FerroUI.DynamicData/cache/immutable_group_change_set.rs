//! `IImmutableGroupChangeSet<TObject, TKey, TGroupKey>` and `ImmutableGroupChangeSet<TObject,
//! TKey, TGroupKey>` (`Cache/IImmutableGroupChangeSet.cs`, `Cache/Internal/ImmutableGroupChangeSet.cs`).

use super::{Change, ChangeSet, IChangeSet, ImmutableGroup};
use std::fmt;
use std::ops::Deref;

/// The changes of the states of the groups of a cache (`IImmutableGroupChangeSet<TObject, TKey,
/// TGroupKey>`, a change set of `IGrouping<TObject, TKey, TGroupKey>` keyed by the group key): a
/// group whose items change is updated with its new state.
///
/// As [`GroupChangeSet`](super::GroupChangeSet), it holds a plain change set of
/// [`ImmutableGroup`]s, which [`Deref`] and [`IChangeSet`] give.
pub struct ImmutableGroupChangeSet<T, K, G> {
    changes: ChangeSet<ImmutableGroup<T, K, G>, G>,
}

impl<T, K, G> Clone for ImmutableGroupChangeSet<T, K, G> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone() }
    }
}

impl<T, K, G> Default for ImmutableGroupChangeSet<T, K, G> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T, K, G> ImmutableGroupChangeSet<T, K, G> {
    /// A change set without changes (`ImmutableGroupChangeSet<TObject, TKey, TGroupKey>.Empty`).
    pub fn empty() -> Self {
        Self { changes: ChangeSet::empty() }
    }

    /// The changes as a plain change set.
    pub fn change_set(&self) -> &ChangeSet<ImmutableGroup<T, K, G>, G> {
        &self.changes
    }
}

impl<T, K, G> From<ChangeSet<ImmutableGroup<T, K, G>, G>> for ImmutableGroupChangeSet<T, K, G> {
    fn from(changes: ChangeSet<ImmutableGroup<T, K, G>, G>) -> Self {
        Self { changes }
    }
}

impl<T, K, G> From<Vec<Change<ImmutableGroup<T, K, G>, G>>> for ImmutableGroupChangeSet<T, K, G> {
    /// `new ImmutableGroupChangeSet(IEnumerable<Change>)`.
    fn from(changes: Vec<Change<ImmutableGroup<T, K, G>, G>>) -> Self {
        Self { changes: ChangeSet::from(changes) }
    }
}

impl<T, K, G> Deref for ImmutableGroupChangeSet<T, K, G> {
    type Target = ChangeSet<ImmutableGroup<T, K, G>, G>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T, K, G> IntoIterator for &'a ImmutableGroupChangeSet<T, K, G> {
    type Item = &'a Change<ImmutableGroup<T, K, G>, G>;
    type IntoIter = std::slice::Iter<'a, Change<ImmutableGroup<T, K, G>, G>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.changes().iter()
    }
}

impl<T: 'static, K: 'static, G: 'static> IChangeSet<ImmutableGroup<T, K, G>, G> for ImmutableGroupChangeSet<T, K, G> {
    fn change_set(&self) -> &ChangeSet<ImmutableGroup<T, K, G>, G> {
        &self.changes
    }
}

impl<T, K, G: PartialEq> PartialEq for ImmutableGroupChangeSet<T, K, G> {
    fn eq(&self, other: &Self) -> bool {
        self.changes == other.changes
    }
}

impl<T: Clone, K: Eq + std::hash::Hash + Clone, G: fmt::Debug> fmt::Debug for ImmutableGroupChangeSet<T, K, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.changes, f)
    }
}
