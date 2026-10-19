//! `AutoRefresh` (`List/ObservableListEx.AutoRefresh.cs`).

macro_rules! auto_refresh_methods {
    () => {
        /// The changes, with a refresh of an item whenever the property of `property_accessor`
        /// changes on it, so that the operators downstream evaluate it again
        /// (`AutoRefresh(Expression<Func<TObject, TProperty>>)`).
        ///
        /// The path observes its own steps, so the items need not be `INotifyPropertyChanged`
        /// themselves (upstream's constraint `TObject : INotifyPropertyChanged`).
        fn auto_refresh<P: crate::binding::IntoPropertyChain<T>>(&self, property_accessor: P) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>
        where
            P::Value: 'static,
        {
            self.auto_refresh_with(property_accessor, None, None, None)
        }

        /// [`auto_refresh`](Self::auto_refresh) that gathers the refreshes of each
        /// `change_set_buffer` into one change set and throttles the changes of the property of
        /// each item by `property_change_throttle`, on `scheduler`
        /// (`AutoRefresh(Expression<Func<TObject, TProperty>>, TimeSpan? changeSetBuffer, TimeSpan? propertyChangeThrottle, IScheduler? scheduler)`).
        fn auto_refresh_with<P: crate::binding::IntoPropertyChain<T>>(
            &self,
            property_accessor: P,
            change_set_buffer: Option<std::time::Duration>,
            property_change_throttle: Option<std::time::Duration>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>
        where
            P::Value: 'static,
        {
            let chain = crate::binding::IntoPropertyChain::into_property_chain(property_accessor);
            let throttle_scheduler = scheduler.clone();
            self.auto_refresh_on_observable_with(
                move |item: &T| {
                    let changed = crate::binding::when_property_changed_of(chain.clone(), item.clone(), false, false);
                    match property_change_throttle {
                        None => changed,
                        Some(throttle) => ferroui_reactive::ObservableExt::throttle_on(
                            &changed,
                            throttle,
                            throttle_scheduler.clone().unwrap_or_else(ferroui_reactive::concurrency::SchedulerDefaults::default_scheduler),
                        ),
                    }
                },
                change_set_buffer,
                scheduler,
            )
        }

        /// The changes, with a refresh of an item whenever any of its properties changes
        /// (`AutoRefresh()`; the overloads of upstream share the name `AutoRefresh`).
        fn auto_refresh_any(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>
        where
            T: crate::binding::NotifyPropertyChangedEx,
        {
            self.auto_refresh_any_with(None, None, None)
        }

        /// [`auto_refresh_any`](Self::auto_refresh_any) that gathers the refreshes of each
        /// `change_set_buffer` into one change set and throttles the changes of the properties of
        /// each item by `property_change_throttle`, on `scheduler`
        /// (`AutoRefresh(TimeSpan? changeSetBuffer, TimeSpan? propertyChangeThrottle, IScheduler? scheduler)`).
        fn auto_refresh_any_with(
            &self,
            change_set_buffer: Option<std::time::Duration>,
            property_change_throttle: Option<std::time::Duration>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>
        where
            T: crate::binding::NotifyPropertyChangedEx,
        {
            let throttle_scheduler = scheduler.clone();
            self.auto_refresh_on_observable_with(
                move |item: &T| {
                    let changed = crate::binding::NotifyPropertyChangedEx::when_any_property_changed(item, &[]);
                    match property_change_throttle {
                        None => changed,
                        Some(throttle) => ferroui_reactive::ObservableExt::throttle_on(
                            &changed,
                            throttle,
                            throttle_scheduler.clone().unwrap_or_else(ferroui_reactive::concurrency::SchedulerDefaults::default_scheduler),
                        ),
                    }
                },
                change_set_buffer,
                scheduler,
            )
        }
    };
}
pub(crate) use auto_refresh_methods;
