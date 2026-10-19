//! `Batch` (`Cache/ObservableCacheEx.Batch.cs`).

macro_rules! batch_methods {
    () => {
        /// The change sets of each `time_span`, as one change set; the windows without change sets
        /// are left out (`Batch(TimeSpan, IScheduler? = null)`, on the default scheduler).
        fn batch(&self, time_span: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.batch_on(time_span, ferroui_reactive::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// [`batch`](Self::batch) on `scheduler` (`Batch(TimeSpan, IScheduler)`).
        fn batch_on(
            &self,
            time_span: std::time::Duration,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::flatten_buffer_result(ferroui_reactive::ObservableExt::buffer_time_on(&self.change_sets(), time_span, scheduler))
        }
    };
}
pub(crate) use batch_methods;
