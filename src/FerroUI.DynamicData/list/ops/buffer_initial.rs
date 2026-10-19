//! `BufferInitial` (`List/ObservableListEx.BufferInitial.cs`).

macro_rules! buffer_initial_methods {
    () => {
        /// The change sets of the first `initial_buffer` after the first one, as one change set,
        /// then the change sets as they come (`BufferInitial(TimeSpan, IScheduler? = null)`, on the
        /// default scheduler).
        fn buffer_initial(&self, initial_buffer: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            self.buffer_initial_on(initial_buffer, ferroui_reactive::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// [`buffer_initial`](Self::buffer_initial) on `scheduler`.
        fn buffer_initial_on(
            &self,
            initial_buffer: std::time::Duration,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            let deferred = crate::list::internal::defer_until_loaded(self.change_sets());
            ferroui_reactive::ObservableExt::publish_with(&deferred, move |shared: std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>| {
                let initial = ferroui_reactive::ObservableExt::take(
                    &crate::list::flatten_buffer_result(ferroui_reactive::ObservableExt::buffer_time_on(&shared, initial_buffer, scheduler.clone())),
                    1,
                );
                ferroui_reactive::ObservableExt::concat(&initial, shared)
            })
        }
    };
}
pub(crate) use buffer_initial_methods;
