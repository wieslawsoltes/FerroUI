//! `FilterImmutable` (`Cache/ObservableCacheEx.FilterImmutable.cs`).

macro_rules! filter_immutable_methods {
    () => {
        /// The items that `predicate` accepts, for items that never change: faster than
        /// [`filter`](Self::filter), as nothing is cached; an update is evaluated by both of its
        /// items, a refresh is not evaluated again, and moves and indexes are dropped
        /// (`FilterImmutable(Func<TObject, bool>, bool suppressEmptyChangeSets = true)`).
        fn filter_immutable(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.filter_immutable_with(predicate, true)
        }

        /// [`filter_immutable`](Self::filter_immutable) that keeps the empty change sets when
        /// `suppress_empty_change_sets` is false (`FilterImmutable(Func<TObject, bool>, bool)`).
        fn filter_immutable_with(
            &self,
            predicate: impl Fn(&T) -> bool + 'static,
            suppress_empty_change_sets: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::filter_immutable(self.change_sets(), std::rc::Rc::new(move |item: &T| Ok(predicate(item))), suppress_empty_change_sets)
        }

        /// [`filter_immutable`](Self::filter_immutable) with a predicate that may fail: a failure
        /// ends the sequence with its error (upstream catches the exception of the predicate).
        fn try_filter_immutable(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.try_filter_immutable_with(predicate, true)
        }

        /// [`try_filter_immutable`](Self::try_filter_immutable) that keeps the empty change sets
        /// when `suppress_empty_change_sets` is false.
        fn try_filter_immutable_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
            suppress_empty_change_sets: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::filter_immutable(self.change_sets(), std::rc::Rc::new(predicate), suppress_empty_change_sets)
        }
    };
}
pub(crate) use filter_immutable_methods;
