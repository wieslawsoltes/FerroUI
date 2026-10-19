//! `AutoRefreshOnObservable` (`Cache/ObservableCacheEx.AutoRefreshOnObservable.cs`).

macro_rules! auto_refresh_on_observable_methods {
    () => {
        /// The changes, with a refresh of an item whenever the sequence `reevaluator` makes for it
        /// produces a value, so that the operators downstream evaluate it again
        /// (`AutoRefreshOnObservable(Func<TObject, IObservable<TAny>>)`).
        fn auto_refresh_on_observable<A: 'static>(
            &self,
            reevaluator: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<A>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.auto_refresh_on_observable_with(reevaluator, None, None)
        }

        /// [`auto_refresh_on_observable`](Self::auto_refresh_on_observable) that gathers the
        /// refreshes of each `change_set_buffer` into one change set, on `scheduler`
        /// (`AutoRefreshOnObservable(Func<TObject, IObservable<TAny>>, TimeSpan? changeSetBuffer, IScheduler? scheduler)`).
        fn auto_refresh_on_observable_with<A: 'static>(
            &self,
            reevaluator: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<A>> + 'static,
            change_set_buffer: Option<std::time::Duration>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.auto_refresh_on_observable_with_key_with(move |item: &T, _: &K| reevaluator(item), change_set_buffer, scheduler)
        }

        /// [`auto_refresh_on_observable`](Self::auto_refresh_on_observable) with a reevaluator
        /// that is given the key (`AutoRefreshOnObservable(Func<TObject, TKey, IObservable<TAny>>)`).
        fn auto_refresh_on_observable_with_key<A: 'static>(
            &self,
            reevaluator: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<A>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.auto_refresh_on_observable_with_key_with(reevaluator, None, None)
        }

        /// [`auto_refresh_on_observable_with_key`](Self::auto_refresh_on_observable_with_key) that
        /// gathers the refreshes of each `change_set_buffer` into one change set, on `scheduler`
        /// (`AutoRefreshOnObservable(Func<TObject, TKey, IObservable<TAny>>, TimeSpan?, IScheduler?)`).
        fn auto_refresh_on_observable_with_key_with<A: 'static>(
            &self,
            reevaluator: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<A>> + 'static,
            change_set_buffer: Option<std::time::Duration>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::auto_refresh(self.change_sets(), std::rc::Rc::new(reevaluator), change_set_buffer, scheduler)
        }
    };
}
pub(crate) use auto_refresh_on_observable_methods;
