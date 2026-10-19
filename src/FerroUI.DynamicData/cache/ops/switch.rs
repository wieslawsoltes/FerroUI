//! `Switch` (`Cache/ObservableCacheEx.Switch.cs`): the change sets of the latest of a sequence of
//! caches.
//!
//! Deviation (DEVIATIONS.md, DynamicData): the sources are a sequence of
//! [`CacheChangeSetSource`]s, so that the overloads on a sequence of caches
//! (`IObservable<IObservableCache>`) and on a sequence of sequences of change sets
//! (`IObservable<IObservable<IChangeSet>>`) are one method of [`SwitchCacheEx`]. Naming: it is
//! `switch_cache`, as `switch` would hide `ObservableExt::switch`.

use super::combine::CacheChangeSetSource;
use crate::cache::internal::switch_cache_change_sets;
use crate::cache::ChangeSet;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// `Switch` of a sequence of caches or of sequences of change sets.
pub trait SwitchCacheEx {
    /// The type of the items.
    type Item: Clone + 'static;
    /// The type of the keys.
    type Key: Eq + std::hash::Hash + Clone + 'static;

    /// The change sets of the latest source: each new source removes the items of the previous
    /// one, then delivers its own; the result completes when the sequence and the latest source
    /// have completed, and an error of either ends it (`Switch(IObservable<IObservableCache>)`,
    /// `Switch(IObservable<IObservable<IChangeSet>>)`).
    fn switch_cache(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>;
}

impl<S: CacheChangeSetSource> SwitchCacheEx for Rc<dyn IObservable<S>> {
    type Item = S::Item;
    type Key = S::Key;

    fn switch_cache(&self) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>> {
        // `sources.Select(cache => cache.Connect()).Switch()` for the caches.
        switch_cache_change_sets(self.select(|source: S| source.source_change_sets()))
    }
}
