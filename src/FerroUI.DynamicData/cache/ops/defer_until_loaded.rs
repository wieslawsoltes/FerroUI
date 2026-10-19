//! `DeferUntilLoaded` (`Cache/ObservableCacheEx.DeferUntilLoaded.cs`).

macro_rules! defer_until_loaded_methods {
    () => {
        /// The change sets from the first one; empty change sets are left out
        /// (`DeferUntilLoaded(IObservable<IChangeSet>)`).
        fn defer_until_loaded(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::defer_until_loaded(self.change_sets())
        }
    };
}
pub(crate) use defer_until_loaded_methods;

/// The form on caches, a provided method of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! defer_until_loaded_source_methods {
    () => {
        /// The items and their changes once the cache has items (`DeferUntilLoaded(IObservableCache)`).
        fn defer_until_loaded(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::defer_until_loaded_cache(self)
        }
    };
}
pub(crate) use defer_until_loaded_source_methods;
