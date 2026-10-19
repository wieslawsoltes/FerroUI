//! `GroupOnPropertyWithImmutableState` (`List/ObservableListEx.GroupOnPropertyWithImmutableState.cs`).

macro_rules! group_on_property_with_immutable_state_methods {
    () => {
        /// The states of the groups of the items by the value of the property `property_selector`,
        /// which is evaluated again when the property changes on an item
        /// (`GroupOnPropertyWithImmutableState(Expression<Func<TObject, TGroup>>)`).
        fn group_on_property_with_immutable_state<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            property_selector: ferroui_system::component_model::PropertyPath<T, G>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<crate::list::ImmutableGroup<T, G>>>> {
            crate::list::internal::group_on_property_with_immutable_state(self.change_sets(), property_selector, None, None)
        }

        /// [`group_on_property_with_immutable_state`](Self::group_on_property_with_immutable_state)
        /// that evaluates the groups once the property has not changed for
        /// `property_changed_throttle`, timed on the default scheduler
        /// (`GroupOnPropertyWithImmutableState(Expression, TimeSpan?)`).
        fn group_on_property_with_immutable_state_with<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            property_selector: ferroui_system::component_model::PropertyPath<T, G>,
            property_changed_throttle: std::time::Duration,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<crate::list::ImmutableGroup<T, G>>>> {
            crate::list::internal::group_on_property_with_immutable_state(self.change_sets(), property_selector, Some(property_changed_throttle), None)
        }

        /// [`group_on_property_with_immutable_state_with`](Self::group_on_property_with_immutable_state_with)
        /// timed on `scheduler` (`GroupOnPropertyWithImmutableState(Expression, TimeSpan?,
        /// IScheduler?)`).
        fn group_on_property_with_immutable_state_with_on<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            property_selector: ferroui_system::component_model::PropertyPath<T, G>,
            property_changed_throttle: std::time::Duration,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<crate::list::ImmutableGroup<T, G>>>> {
            crate::list::internal::group_on_property_with_immutable_state(self.change_sets(), property_selector, Some(property_changed_throttle), Some(scheduler))
        }
    };
}
pub(crate) use group_on_property_with_immutable_state_methods;
