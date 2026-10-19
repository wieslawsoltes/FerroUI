//! `ExpireAfter` (`Cache/ObservableCacheEx.ExpireAfter.cs`).

macro_rules! expire_after_methods {
    () => {
        /// The items, each removed once the time that `time_selector` gives for it has passed
        /// (`None`: never), timed on the default scheduler; the sequence ends once the source has and
        /// no expiration is pending (`ExpireAfter(Func<TObject, TimeSpan?>)`).
        fn expire_after(&self, time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::expire_after_for_stream(self.change_sets(), std::rc::Rc::new(time_selector), None, None)
        }

        /// [`expire_after`](Self::expire_after) on `scheduler` (`ExpireAfter(.., IScheduler)`).
        fn expire_after_on(
            &self,
            time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::expire_after_for_stream(self.change_sets(), std::rc::Rc::new(time_selector), None, Some(scheduler))
        }

        /// [`expire_after`](Self::expire_after) that removes the expired items every
        /// `polling_interval` rather than at their times (`ExpireAfter(.., TimeSpan?)`).
        fn expire_after_with_polling(
            &self,
            time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static,
            polling_interval: Option<std::time::Duration>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::expire_after_for_stream(self.change_sets(), std::rc::Rc::new(time_selector), polling_interval, None)
        }

        /// [`expire_after_with_polling`](Self::expire_after_with_polling) on `scheduler`
        /// (`ExpireAfter(.., TimeSpan?, IScheduler)`).
        fn expire_after_with_polling_on(
            &self,
            time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static,
            polling_interval: Option<std::time::Duration>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::expire_after_for_stream(self.change_sets(), std::rc::Rc::new(time_selector), polling_interval, Some(scheduler))
        }
    };
}
pub(crate) use expire_after_methods;

/// The forms on source caches, provided methods of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! expire_after_source_methods {
    () => {
        /// Removes each item from the cache once the time that `time_selector` gives for it has
        /// passed (`None`: never), timed on the default scheduler; the removed items with their keys
        /// (`ExpireAfter(ISourceCache, Func<TObject, TimeSpan?>, TimeSpan? = null, IScheduler? =
        /// null)`).
        fn expire_after(&self, time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<(K, T)>>>
        where
            Self: crate::cache::ISourceCache<T, K> + Clone + Sized + 'static,
        {
            crate::cache::internal::expire_after_for_source(self.clone(), std::rc::Rc::new(time_selector), None, None)
        }

        /// [`expire_after`](Self::expire_after) on `scheduler`.
        fn expire_after_on(
            &self,
            time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<(K, T)>>>
        where
            Self: crate::cache::ISourceCache<T, K> + Clone + Sized + 'static,
        {
            crate::cache::internal::expire_after_for_source(self.clone(), std::rc::Rc::new(time_selector), None, Some(scheduler))
        }

        /// [`expire_after`](Self::expire_after) that removes the expired items every
        /// `polling_interval` rather than at their times.
        fn expire_after_with_polling(
            &self,
            time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static,
            polling_interval: Option<std::time::Duration>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<(K, T)>>>
        where
            Self: crate::cache::ISourceCache<T, K> + Clone + Sized + 'static,
        {
            crate::cache::internal::expire_after_for_source(self.clone(), std::rc::Rc::new(time_selector), polling_interval, None)
        }

        /// [`expire_after_with_polling`](Self::expire_after_with_polling) on `scheduler`.
        fn expire_after_with_polling_on(
            &self,
            time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static,
            polling_interval: Option<std::time::Duration>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<(K, T)>>>
        where
            Self: crate::cache::ISourceCache<T, K> + Clone + Sized + 'static,
        {
            crate::cache::internal::expire_after_for_source(self.clone(), std::rc::Rc::new(time_selector), polling_interval, Some(scheduler))
        }
    };
}
pub(crate) use expire_after_source_methods;
