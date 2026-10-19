//! `LimitSizeTo` (`List/ObservableListEx.LimitSizeTo.cs`).

/// The forms on source lists, provided methods of [`SourceListEx`](crate::list::SourceListEx).
macro_rules! limit_size_to_source_methods {
    () => {
        /// Removes the oldest items of the list beyond `size_limit`, on the default scheduler; the
        /// removed items (`LimitSizeTo(ISourceList, int, IScheduler? = null)`).
        ///
        /// # Panics
        /// For a size of zero (`ArgumentException`).
        fn limit_size_to(&self, size_limit: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            Self: crate::list::ISourceList<T> + Clone + Sized + 'static,
        {
            crate::list::internal::limit_size_to(self.clone(), size_limit, ferroui_reactive::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// [`limit_size_to`](Self::limit_size_to) on `scheduler`.
        fn limit_size_to_on(&self, size_limit: usize, scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            Self: crate::list::ISourceList<T> + Clone + Sized + 'static,
        {
            crate::list::internal::limit_size_to(self.clone(), size_limit, scheduler)
        }
    };
}
pub(crate) use limit_size_to_source_methods;
