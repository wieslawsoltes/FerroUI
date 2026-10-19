//! `FilterOnObservable` (`Cache/ObservableCacheEx.FilterOnObservable.cs`).

macro_rules! filter_on_observable_methods {
    () => {
        /// The items whose sequence of `filter_factory` produced `true` last; an item is excluded
        /// until its sequence produces a value (`FilterOnObservable(Func<TObject, IObservable<bool>>)`).
        fn filter_on_observable(
            &self,
            filter_factory: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.filter_on_observable_with(filter_factory, None, None)
        }

        /// [`filter_on_observable`](Self::filter_on_observable) that gathers the changes of the
        /// filters of each `buffer` into one change set, on `scheduler`
        /// (`FilterOnObservable(Func<TObject, IObservable<bool>>, TimeSpan? buffer, IScheduler? scheduler)`).
        fn filter_on_observable_with(
            &self,
            filter_factory: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> + 'static,
            buffer: Option<std::time::Duration>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.filter_on_observable_with_key_with(move |item: &T, _: &K| filter_factory(item), buffer, scheduler)
        }

        /// [`filter_on_observable`](Self::filter_on_observable) with a factory that is given the
        /// key (`FilterOnObservable(Func<TObject, TKey, IObservable<bool>>)`).
        fn filter_on_observable_with_key(
            &self,
            filter_factory: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.filter_on_observable_with_key_with(filter_factory, None, None)
        }

        /// [`filter_on_observable_with_key`](Self::filter_on_observable_with_key) that gathers the
        /// changes of the filters of each `buffer` into one change set, on `scheduler`
        /// (`FilterOnObservable(Func<TObject, TKey, IObservable<bool>>, TimeSpan?, IScheduler?)`).
        fn filter_on_observable_with_key_with(
            &self,
            filter_factory: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> + 'static,
            buffer: Option<std::time::Duration>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::filter_on_observable(self.change_sets(), std::rc::Rc::new(filter_factory), buffer, scheduler)
        }
    };
}
pub(crate) use filter_on_observable_methods;
