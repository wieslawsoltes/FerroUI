//! `GroupOnProperty` (`Cache/ObservableCacheEx.GroupOnProperty.cs`).

macro_rules! group_on_property_methods {
    () => {
        /// The items grouped by the value of the property `property_selector`, which is evaluated
        /// again when the property changes on an item (`GroupOnProperty(Expression<Func<TObject,
        /// TGroupKey>>)`).
        fn group_on_property<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            property_selector: ferroui_system::component_model::PropertyPath<T, G>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_property(self.change_sets(), property_selector, None, None)
        }

        /// [`group_on_property`](Self::group_on_property) that evaluates the groups once the
        /// property has not changed for `property_changed_throttle`, timed on the default scheduler
        /// (`GroupOnProperty(Expression, TimeSpan?)`).
        fn group_on_property_with<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            property_selector: ferroui_system::component_model::PropertyPath<T, G>,
            property_changed_throttle: std::time::Duration,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_property(self.change_sets(), property_selector, Some(property_changed_throttle), None)
        }

        /// [`group_on_property_with`](Self::group_on_property_with) timed on `scheduler`
        /// (`GroupOnProperty(Expression, TimeSpan?, IScheduler?)`).
        fn group_on_property_with_on<G: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            property_selector: ferroui_system::component_model::PropertyPath<T, G>,
            property_changed_throttle: std::time::Duration,
            scheduler: std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::GroupChangeSet<T, K, G>>> {
            crate::cache::internal::group_on_property(self.change_sets(), property_selector, Some(property_changed_throttle), Some(scheduler))
        }
    };
}
pub(crate) use group_on_property_methods;
