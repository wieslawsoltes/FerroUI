//! `IGroupChangeSet<TObject, TKey, TGroupKey>` and `GroupChangeSet<TObject, TKey, TGroupKey>`
//! (`Cache/IGroupChangeSet.cs`, `Cache/GroupChangeSet.cs`).

use super::{Change, ChangeSet, Group, IChangeSet};
use std::fmt;
use std::ops::Deref;

/// The changes of the groups of a cache (`IGroupChangeSet<TObject, TKey, TGroupKey>`, a change set
/// of `IGroup<TObject, TKey, TGroupKey>` keyed by the group key): the adds and removes of groups,
/// whose items change through the cache of each group.
///
/// It holds a plain change set of [`Group`]s, which [`Deref`] and [`IChangeSet`] give, so the
/// operators of caches apply to a sequence of groups as upstream's do through the interface
/// (docs/porting/reactive.md, section 5.4).
pub struct GroupChangeSet<T, K, G> {
    changes: ChangeSet<Group<T, K, G>, G>,
}

impl<T, K, G> Clone for GroupChangeSet<T, K, G> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone() }
    }
}

impl<T, K, G> Default for GroupChangeSet<T, K, G> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T, K, G> GroupChangeSet<T, K, G> {
    /// A change set without changes (`GroupChangeSet<TObject, TKey, TGroupKey>.Empty`).
    pub fn empty() -> Self {
        Self { changes: ChangeSet::empty() }
    }

    /// The changes as a plain change set.
    pub fn change_set(&self) -> &ChangeSet<Group<T, K, G>, G> {
        &self.changes
    }
}

impl<T, K, G> From<ChangeSet<Group<T, K, G>, G>> for GroupChangeSet<T, K, G> {
    /// The change set of groups of `changes` (`new GroupChangeSet(IEnumerable<Change>)`).
    fn from(changes: ChangeSet<Group<T, K, G>, G>) -> Self {
        Self { changes }
    }
}

impl<T, K, G> From<Vec<Change<Group<T, K, G>, G>>> for GroupChangeSet<T, K, G> {
    fn from(changes: Vec<Change<Group<T, K, G>, G>>) -> Self {
        Self { changes: ChangeSet::from(changes) }
    }
}

impl<T, K, G> Deref for GroupChangeSet<T, K, G> {
    type Target = ChangeSet<Group<T, K, G>, G>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T, K, G> IntoIterator for &'a GroupChangeSet<T, K, G> {
    type Item = &'a Change<Group<T, K, G>, G>;
    type IntoIter = std::slice::Iter<'a, Change<Group<T, K, G>, G>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.changes().iter()
    }
}

impl<T: 'static, K: 'static, G: 'static> IChangeSet<Group<T, K, G>, G> for GroupChangeSet<T, K, G> {
    fn change_set(&self) -> &ChangeSet<Group<T, K, G>, G> {
        &self.changes
    }
}

impl<T, K, G: PartialEq> PartialEq for GroupChangeSet<T, K, G> {
    fn eq(&self, other: &Self) -> bool {
        self.changes == other.changes
    }
}

impl<T, K, G: fmt::Debug> fmt::Debug for GroupChangeSet<T, K, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.changes, f)
    }
}
