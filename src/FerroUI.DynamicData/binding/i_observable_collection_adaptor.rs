//! `IObservableCollectionAdaptor<TObject, TKey>` and `ISortedObservableCollectionAdaptor<TObject,
//! TKey>` (`Binding/IObservableCollectionAdaptor.cs`, `Binding/ISortedObservableCollectionAdaptor.cs`).

use super::IObservableCollection;
use crate::cache::{ChangeSet, SortedChangeSet};

/// Applies the change sets of a cache to an observable collection
/// (`IObservableCollectionAdaptor<TObject, TKey>`).
pub trait IObservableCollectionAdaptor<T: Clone + PartialEq + 'static, K> {
    /// Applies `changes` to `collection` (`Adapt`).
    fn adapt(&self, changes: &ChangeSet<T, K>, collection: &dyn IObservableCollection<T>);
}

/// Applies the sorted change sets of a cache to an observable collection
/// (`ISortedObservableCollectionAdaptor<TObject, TKey>`).
pub trait ISortedObservableCollectionAdaptor<T: Clone + PartialEq + 'static, K> {
    /// Applies `changes` to `collection` (`Adapt`).
    fn adapt(&self, changes: &SortedChangeSet<T, K>, collection: &dyn IObservableCollection<T>);
}
