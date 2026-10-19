//! `FilterOnObservable` (`List/ObservableListEx.FilterOnObservable.cs`).

macro_rules! filter_on_observable_methods {
    () => {
        /// The items whose sequence of `object_filter_observable` produced `true` last; an item is
        /// included until its sequence produces `false`
        /// (`FilterOnObservable(Func<TObject, IObservable<bool>>)`).
        fn filter_on_observable(
            &self,
            object_filter_observable: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            self.filter_on_observable_with(object_filter_observable, None, None)
        }

        /// [`filter_on_observable`](Self::filter_on_observable) that gathers the changes of the
        /// filters of each `property_changed_throttle` into one change set, on `scheduler`
        /// (`FilterOnObservable(Func<TObject, IObservable<bool>>, TimeSpan? propertyChangedThrottle, IScheduler? scheduler)`).
        fn filter_on_observable_with(
            &self,
            object_filter_observable: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> + 'static,
            property_changed_throttle: Option<std::time::Duration>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::filter_on_observable(self.change_sets(), std::rc::Rc::new(object_filter_observable), property_changed_throttle, scheduler)
        }
    };
}
pub(crate) use filter_on_observable_methods;
