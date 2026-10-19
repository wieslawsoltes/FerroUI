//! `GroupOnObservable` (`Cache/ObservableCacheEx.GroupOnObservable.cs`).

macro_rules! group_on_observable_methods {
    () => {
        /// The items grouped by the latest value of the sequence of group keys that
        /// `group_observable_selector` gives for each item; the result ends when the source and
        /// every sequence of group keys have ended
        /// (`GroupOnObservable(Func<TObject, IObservable<TGroupKey>>)`).
        fn group_on_observable<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<G>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_observable(self.change_sets(), std::rc::Rc::new(move |item: &T, _: &K| Ok::<_, ferroui_system::ObservableError>(group_observable_selector(item))))
        }

        /// [`group_on_observable`](Self::group_on_observable) with a selector of the item and its
        /// key (`GroupOnObservable(Func<TObject, TKey, IObservable<TGroupKey>>)`).
        fn group_on_observable_with_key<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_observable_selector: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<G>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_observable(self.change_sets(), std::rc::Rc::new(move |item: &T, key: &K| Ok::<_, ferroui_system::ObservableError>(group_observable_selector(item, key))))
        }

        /// [`group_on_observable`](Self::group_on_observable) with a selector that may fail; a
        /// failure ends the result with its error (`GroupOnObservable` with a selector that
        /// throws).
        fn try_group_on_observable<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_observable_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<G>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_observable(self.change_sets(), std::rc::Rc::new(move |item: &T, _: &K| group_observable_selector(item)))
        }

        /// [`group_on_observable_with_key`](Self::group_on_observable_with_key) with a selector that
        /// may fail; a failure ends the result with its error.
        fn try_group_on_observable_with_key<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_observable_selector: impl Fn(&T, &K) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<G>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_observable(self.change_sets(), std::rc::Rc::new(group_observable_selector))
        }
    };
}
pub(crate) use group_on_observable_methods;
