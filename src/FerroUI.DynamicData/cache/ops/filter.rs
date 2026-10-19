//! `Filter` (`Cache/ObservableCacheEx.Filter.cs`): the static, dynamic and state forms.

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

        /// The items that the latest predicate of `predicate_changed` accepts: each new predicate
        /// evaluates every item again; until the first, no item is included
        /// (`Filter(IObservable<Func<TObject, bool>>, bool suppressEmptyChangeSets = true)`).
        fn filter_dynamic(
            &self,
            predicate_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.filter_dynamic_reapply_with(predicate_changed, ferroui_reactive::Observable::empty(), true)
        }

        /// [`filter_dynamic`](Self::filter_dynamic) that keeps the empty change sets when
        /// `suppress_empty_change_sets` is false
        /// (`Filter(IObservable<Func<TObject, bool>>, bool)`).
        fn filter_dynamic_with(
            &self,
            predicate_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> bool>>>,
            suppress_empty_change_sets: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.filter_dynamic_reapply_with(predicate_changed, ferroui_reactive::Observable::empty(), suppress_empty_change_sets)
        }

        /// [`filter_dynamic`](Self::filter_dynamic) that also evaluates every item again with the
        /// latest predicate when `reapply_filter` produces a value
        /// (`Filter(IObservable<Func<TObject, bool>>, IObservable<Unit>, bool suppressEmptyChangeSets = true)`).
        fn filter_dynamic_reapply(
            &self,
            predicate_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> bool>>>,
            reapply_filter: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.filter_dynamic_reapply_with(predicate_changed, reapply_filter, true)
        }

        /// [`filter_dynamic_reapply`](Self::filter_dynamic_reapply) that keeps the empty change sets
        /// when `suppress_empty_change_sets` is false
        /// (`Filter(IObservable<Func<TObject, bool>>, IObservable<Unit>, bool)`).
        fn filter_dynamic_reapply_with(
            &self,
            predicate_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> bool>>>,
            reapply_filter: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
            suppress_empty_change_sets: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::filter_dynamic(
                self.change_sets(),
                predicate_changed,
                std::rc::Rc::new(|predicate: &std::rc::Rc<dyn Fn(&T) -> bool>, item: &T| predicate(item)),
                reapply_filter,
                suppress_empty_change_sets,
            )
        }

        /// The items that `predicate` accepts with the latest value of `predicate_state`: each new
        /// value evaluates every item again; until the first, no item is included
        /// (`Filter<TObject, TKey, TState>(IObservable<TState>, Func<TState, TObject, bool>, bool suppressEmptyChangeSets = true)`).
        fn filter_with_state<S: 'static>(
            &self,
            predicate_state: std::rc::Rc<dyn ferroui_system::IObservable<S>>,
            predicate: impl Fn(&S, &T) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            self.filter_with_state_with(predicate_state, predicate, true)
        }

        /// [`filter_with_state`](Self::filter_with_state) that keeps the empty change sets when
        /// `suppress_empty_change_sets` is false
        /// (`Filter<TObject, TKey, TState>(IObservable<TState>, Func<TState, TObject, bool>, bool)`).
        fn filter_with_state_with<S: 'static>(
            &self,
            predicate_state: std::rc::Rc<dyn ferroui_system::IObservable<S>>,
            predicate: impl Fn(&S, &T) -> bool + 'static,
            suppress_empty_change_sets: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::filter_dynamic(
                self.change_sets(),
                predicate_state,
                std::rc::Rc::new(predicate),
                ferroui_reactive::Observable::empty(),
                suppress_empty_change_sets,
            )
        }
    };
}
pub(crate) use filter_methods;
