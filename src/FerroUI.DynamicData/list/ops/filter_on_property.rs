//! `FilterOnProperty` (`List/ObservableListEx.FilterOnProperty.cs`), which upstream marks obsolete
//! ("Use AutoRefresh(), followed by Filter() instead").

macro_rules! filter_on_property_methods {
    () => {
        /// The items that `predicate` accepts, evaluated again whenever the property of
        /// `property_selector` changes on an item (`FilterOnProperty(Expression<Func<TObject, TProperty>>, Func<TObject, bool>)`:
        /// `AutoRefresh(propertySelector)` followed by `Filter(predicate)`).
        fn filter_on_property<P: crate::binding::IntoPropertyChain<T>>(
            &self,
            property_selector: P,
            predicate: impl Fn(&T) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>
        where
            P::Value: 'static,
        {
            self.filter_on_property_with(property_selector, predicate, None, None)
        }

        /// [`filter_on_property`](Self::filter_on_property) that throttles the changes of the
        /// property of each item by `property_changed_throttle`, on `scheduler`
        /// (`FilterOnProperty(.., TimeSpan? propertyChangedThrottle, IScheduler? scheduler)`).
        fn filter_on_property_with<P: crate::binding::IntoPropertyChain<T>>(
            &self,
            property_selector: P,
            predicate: impl Fn(&T) -> bool + 'static,
            property_changed_throttle: Option<std::time::Duration>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>
        where
            P::Value: 'static,
        {
            let refreshed = self.auto_refresh_with(property_selector, None, property_changed_throttle, scheduler);
            <std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> as crate::list::ObservableListEx<T>>::filter(&refreshed, predicate)
        }
    };
}
pub(crate) use filter_on_property_methods;
