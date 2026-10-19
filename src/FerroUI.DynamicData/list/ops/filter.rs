//! `Filter` (`List/ObservableListEx.Filter.cs`): the static, dynamic and state forms.

macro_rules! filter_methods {
    () => {
        /// The items that `predicate` accepts, in the order of the source; a refresh or a
        /// replacement evaluates the item again (`Filter(Func<T, bool>)`).
        fn filter(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::static_filter(self.change_sets(), std::rc::Rc::new(predicate), true)
        }

        /// The items that the latest predicate of `predicate` accepts; until the first, none is. A
        /// new predicate removes the items that no longer match and adds those that now do
        /// (`Filter(IObservable<Func<T, bool>>, ListFilterPolicy filterPolicy = CalculateDiff)`).
        fn filter_dynamic(
            &self,
            predicate: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            self.filter_dynamic_with(predicate, crate::list::ListFilterPolicy::CalculateDiff)
        }

        /// [`filter_dynamic`](Self::filter_dynamic) with the policy of a new predicate
        /// (`Filter(IObservable<Func<T, bool>>, ListFilterPolicy)`).
        fn filter_dynamic_with(
            &self,
            predicate: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> bool>>>,
            filter_policy: crate::list::ListFilterPolicy,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::dynamic_filter(self.change_sets(), predicate, filter_policy)
        }

        /// The items that `predicate` accepts with the latest value of `predicate_state`, in the
        /// order of the source; until the first value, none is
        /// (`Filter<T, TState>(IObservable<TState>, Func<TState, T, bool>, ListFilterPolicy = CalculateDiff, bool suppressEmptyChangeSets = true)`).
        fn filter_with_state<S: 'static>(
            &self,
            predicate_state: std::rc::Rc<dyn ferroui_system::IObservable<S>>,
            predicate: impl Fn(&S, &T) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            self.filter_with_state_with(predicate_state, predicate, crate::list::ListFilterPolicy::CalculateDiff, true)
        }

        /// [`filter_with_state`](Self::filter_with_state) with the policy of a new state, keeping
        /// the empty change sets when `suppress_empty_change_sets` is false
        /// (`Filter<T, TState>(IObservable<TState>, Func<TState, T, bool>, ListFilterPolicy, bool)`).
        fn filter_with_state_with<S: 'static>(
            &self,
            predicate_state: std::rc::Rc<dyn ferroui_system::IObservable<S>>,
            predicate: impl Fn(&S, &T) -> bool + 'static,
            filter_policy: crate::list::ListFilterPolicy,
            suppress_empty_change_sets: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::filter_with_predicate_state(self.change_sets(), predicate_state, std::rc::Rc::new(predicate), filter_policy, suppress_empty_change_sets)
        }
    };
}
pub(crate) use filter_methods;
