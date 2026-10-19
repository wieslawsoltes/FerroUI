//! `IChangeSetAdaptor<TObject, TKey>` and `ISortedChangeSetAdaptor<TObject, TKey>`
//! (`Cache/IChangeSetAdaptor.cs`, `Cache/ISortedChangeSetAdaptor.cs`).

use super::{ChangeSet, SortedChangeSet};

/// Applies the change sets of a cache to something else (`IChangeSetAdaptor<TObject, TKey>`).
pub trait IChangeSetAdaptor<T, K> {
    /// Applies `changes` (`Adapt`).
    fn adapt(&self, changes: &ChangeSet<T, K>);
}

/// Applies the sorted change sets of a cache to something else
/// (`ISortedChangeSetAdaptor<TObject, TKey>`).
pub trait ISortedChangeSetAdaptor<T, K> {
    /// Applies `changes` (`Adapt`).
    fn adapt(&self, changes: &SortedChangeSet<T, K>);
}
