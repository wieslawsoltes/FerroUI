//! `WatchValue` (`Cache/ObservableCacheEx.WatchValue.cs`).

macro_rules! watch_value_methods {
    () => {
        /// The item of `key` each time it changes (`WatchValue(IObservable<IChangeSet>, TKey)`).
        fn watch_value(&self, key: K) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            ferroui_reactive::ObservableExt::select(&self.watch(key), |update: crate::cache::Change<T, K>| update.current().clone())
        }
    };
}
pub(crate) use watch_value_methods;

/// The form on caches, a provided method of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! watch_value_source_methods {
    () => {
        /// The item of `key` each time it changes (`WatchValue(IObservableCache, TKey)`).
        fn watch_value(&self, key: K) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            ferroui_reactive::ObservableExt::select(&crate::cache::IConnectableCache::watch(self, key), |update: crate::cache::Change<T, K>| update.current().clone())
        }
    };
}
pub(crate) use watch_value_source_methods;
