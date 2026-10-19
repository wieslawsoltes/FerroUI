//! `LimitSizeTo` (`Cache/ObservableCacheEx.LimitSizeTo.cs`).

macro_rules! limit_size_to_methods {
    () => {
        /// The items, the oldest removed beyond `size` (`LimitSizeTo(IObservable<IChangeSet>, int)`).
        ///
        /// # Panics
        /// For a size of zero (`ArgumentException`).
        fn limit_size_to(&self, size: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::size_expirer(self.change_sets(), size)
        }
    };
}
pub(crate) use limit_size_to_methods;

/// The forms on source caches, provided methods of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! limit_size_to_source_methods {
    () => {
        /// Removes the oldest items of the cache beyond `size_limit`, on the default scheduler; the
        /// removed items with their keys (`LimitSizeTo(ISourceCache, int, IScheduler? = null)`).
        ///
        /// # Panics
        /// For a size of zero (`ArgumentException`).
        fn limit_size_to(&self, size_limit: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<(K, T)>>>
        where
            Self: crate::cache::ISourceCache<T, K> + Clone + Sized + 'static,
        {
            crate::cache::internal::limit_size_to_source(self.clone(), size_limit, None)
        }

        /// [`limit_size_to`](Self::limit_size_to) on `scheduler` (`LimitSizeTo(ISourceCache, int,
        /// IScheduler)`).
        fn limit_size_to_on(&self, size_limit: usize, scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<(K, T)>>>
        where
            Self: crate::cache::ISourceCache<T, K> + Clone + Sized + 'static,
        {
            crate::cache::internal::limit_size_to_source(self.clone(), size_limit, Some(scheduler))
        }
    };
}
pub(crate) use limit_size_to_source_methods;
