//! `ChangeSet<TObject, TKey, TContext>` and `IChangeSet<TObject, TKey, TContext>`
//! (`Cache/ChangeSet.cs`, `Cache/IChangeSet.cs`): a change set with a context.

use super::{Change, ChangeSet, IChangeSet};
use std::fmt;
use std::ops::Deref;

/// The changes of a cache with a context (`ChangeSet<TObject, TKey, TContext>`, which
/// implements `IChangeSet<TObject, TKey, TContext>`): the change sets of `SortAndPage`
/// ([`PageContext`](super::PageContext)) and of `SortAndVirtualize`
/// ([`VirtualContext`](super::VirtualContext)).
///
/// Deviation (DEVIATIONS.md, DynamicData): Rust has no overloading of a type by the number of its
/// parameters, so upstream's `ChangeSet<TObject, TKey, TContext>` is named
/// `ChangeSetWithContext`. It dereferences to its [`ChangeSet`], as upstream's derives from it.
pub struct ChangeSetWithContext<T, K, X> {
    changes: ChangeSet<T, K>,
    context: X,
}

impl<T, K, X: Clone> Clone for ChangeSetWithContext<T, K, X> {
    fn clone(&self) -> Self {
        Self { changes: self.changes.clone(), context: self.context.clone() }
    }
}

impl<T, K, X> ChangeSetWithContext<T, K, X> {
    /// The changes `collection` with `context` (`ChangeSet(IEnumerable<Change>, TContext)`).
    pub fn new(collection: ChangeSet<T, K>, context: X) -> Self {
        Self { changes: collection, context }
    }

    /// A change set without changes, with `context` (`ChangeSet(TContext)`).
    pub fn empty(context: X) -> Self {
        Self::new(ChangeSet::empty(), context)
    }

    /// The context (`Context`).
    pub fn context(&self) -> &X {
        &self.context
    }

    /// The changes.
    pub fn changes(&self) -> &ChangeSet<T, K> {
        &self.changes
    }
}

impl<T, K, X> Deref for ChangeSetWithContext<T, K, X> {
    type Target = ChangeSet<T, K>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T, K, X> IntoIterator for &'a ChangeSetWithContext<T, K, X> {
    type Item = &'a Change<T, K>;
    type IntoIter = std::slice::Iter<'a, Change<T, K>>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.changes().iter()
    }
}

impl<T: 'static, K: 'static, X: Clone + 'static> IChangeSet<T, K> for ChangeSetWithContext<T, K, X> {
    fn change_set(&self) -> &ChangeSet<T, K> {
        &self.changes
    }
}

impl<T: fmt::Debug, K: fmt::Debug, X: fmt::Debug> fmt::Debug for ChangeSetWithContext<T, K, X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChangeSetWithContext").field("changes", &self.changes).field("context", &self.context).finish()
    }
}
