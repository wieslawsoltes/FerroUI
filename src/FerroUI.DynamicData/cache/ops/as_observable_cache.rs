//! `AsObservableCache` (`Cache/ObservableCacheEx.AsObservableCache.cs`).

macro_rules! as_observable_cache_methods {
    () => {
        /// A cache of the items of the sequence, which applies its change sets
        /// (`AsObservableCache(IObservable<IChangeSet>, bool applyLocking = true)`; there is no
        /// locking on one thread).
        fn as_observable_cache(&self) -> crate::cache::ObservableCache<T, K> {
            crate::cache::ObservableCache::from_source(self.change_sets())
        }
    };
}
pub(crate) use as_observable_cache_methods;
