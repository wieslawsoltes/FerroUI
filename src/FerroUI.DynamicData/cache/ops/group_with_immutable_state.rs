//! `GroupWithImmutableState` (`Cache/ObservableCacheEx.GroupWithImmutableState.cs`).

macro_rules! group_with_immutable_state_methods {
    () => {
        /// The items grouped by the key `group_selector_key` selects, as states of the groups that
        /// do not change: a group whose items change is updated with its new state
        /// (`GroupWithImmutableState(Func<TObject, TGroupKey>, IObservable<Unit>? regrouper =
        /// null)`).
        fn group_with_immutable_state<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector_key: impl Fn(&T) -> G + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ImmutableGroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_immutable(self.change_sets(), std::rc::Rc::new(group_selector_key), None)
        }

        /// [`group_with_immutable_state`](Self::group_with_immutable_state) that evaluates the group
        /// of every item again when `regrouper` produces a value
        /// (`GroupWithImmutableState(Func<TObject, TGroupKey>, IObservable<Unit>)`).
        fn group_with_immutable_state_with<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            group_selector_key: impl Fn(&T) -> G + 'static,
            regrouper: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ImmutableGroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_immutable(self.change_sets(), std::rc::Rc::new(group_selector_key), Some(regrouper))
        }
    };
}
pub(crate) use group_with_immutable_state_methods;
