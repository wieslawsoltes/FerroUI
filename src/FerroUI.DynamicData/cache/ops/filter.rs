//! `Filter` (`Cache/ObservableCacheEx.Filter.cs`): the static forms.

macro_rules! filter_methods {
    () => {
        /// The items that `filter` accepts; an update or a refresh evaluates the item again
        /// (`Filter(Func<TObject, bool>, bool suppressEmptyChangeSets = true)`).
        fn filter(&self, filter: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::filter_static(self.change_sets(), std::rc::Rc::new(filter), true)
        }

        /// [`filter`](Self::filter) that keeps the empty change sets when
        /// `suppress_empty_change_sets` is false (`Filter(Func<TObject, bool>, bool)`).
        fn filter_with(
            &self,
            filter: impl Fn(&T) -> bool + 'static,
            suppress_empty_change_sets: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::filter_static(self.change_sets(), std::rc::Rc::new(filter), suppress_empty_change_sets)
        }
    };
}
pub(crate) use filter_methods;
