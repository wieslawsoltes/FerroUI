//! `ExpireAfter` (`List/ObservableListEx.ExpireAfter.cs`).

/// The forms on source lists, provided methods of [`SourceListEx`](crate::list::SourceListEx).
macro_rules! expire_after_source_methods {
    () => {
        /// Removes each item from the list once the time that `time_selector` gives for it has
        /// passed (`None`: never), timed on the default scheduler; the removed items
        /// (`ExpireAfter(ISourceList, Func<T, TimeSpan?>, TimeSpan? = null, IScheduler? = null)`).
        fn expire_after(&self, time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            Self: crate::list::ISourceList<T> + Clone + Sized + 'static,
        {
            crate::list::internal::expire_after(self.clone(), std::rc::Rc::new(time_selector), None, None)
        }

        /// [`expire_after`](Self::expire_after) on `scheduler`.
        fn expire_after_on(
            &self,
            time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            Self: crate::list::ISourceList<T> + Clone + Sized + 'static,
        {
            crate::list::internal::expire_after(self.clone(), std::rc::Rc::new(time_selector), None, Some(scheduler))
        }

        /// [`expire_after`](Self::expire_after) that removes the expired items every
        /// `polling_interval` rather than at their times.
        fn expire_after_with_polling(
            &self,
            time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static,
            polling_interval: Option<std::time::Duration>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            Self: crate::list::ISourceList<T> + Clone + Sized + 'static,
        {
            crate::list::internal::expire_after(self.clone(), std::rc::Rc::new(time_selector), polling_interval, None)
        }

        /// [`expire_after_with_polling`](Self::expire_after_with_polling) on `scheduler`.
        fn expire_after_with_polling_on(
            &self,
            time_selector: impl Fn(&T) -> Option<std::time::Duration> + 'static,
            polling_interval: Option<std::time::Duration>,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            Self: crate::list::ISourceList<T> + Clone + Sized + 'static,
        {
            crate::list::internal::expire_after(self.clone(), std::rc::Rc::new(time_selector), polling_interval, Some(scheduler))
        }
    };
}
pub(crate) use expire_after_source_methods;
