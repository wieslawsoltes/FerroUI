//! `IGroup<TObject, TKey, TGroupKey>` (`Cache/IGroup.cs`).

use super::IObservableCache;
use std::rc::Rc;

/// A group of the items of a cache that share a group key, with the cache of those items
/// (`IGroup<TObject, TKey, TGroupKey>`): the items of the change sets of `Group`,
/// `GroupOnObservable` and `GroupOnProperty`.
///
/// Deviation (DEVIATIONS.md, DynamicData): the change sets of groups hold the concrete
/// [`Group`](super::Group), which implements this trait, rather than the interface: a change set
/// holds its items by value.
pub trait IGroup<T, K, G> {
    /// The group key (`Key`, of `IKey<TGroupKey>`).
    fn key(&self) -> &G;

    /// The items of the group, as a cache that can be observed (`Cache`).
    fn cache(&self) -> Rc<dyn IObservableCache<T, K>>;
}
