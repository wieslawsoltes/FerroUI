//! `WhenPropertyChanged` (`List/ObservableListEx.WhenPropertyChanged.cs`).

macro_rules! when_property_changed_methods {
    () => {
        /// The changes of the property of `property_accessor` of every item, as the item with the
        /// value, the current values first, merged
        /// (`WhenPropertyChanged(Expression<Func<TObject, TValue>>, bool notifyOnInitialValue = true)`).
        ///
        /// Unlike the cache's, upstream's list form keeps the values that a null step of the path
        /// interrupts ([`is_unobtainable`](crate::binding::PropertyValue::is_unobtainable)).
        fn when_property_changed<P: crate::binding::IntoPropertyChain<T>>(
            &self,
            property_accessor: P,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::binding::PropertyValue<T, P::Value>>>
        where
            P::Value: 'static,
        {
            self.when_property_changed_with(property_accessor, true)
        }

        /// [`when_property_changed`](Self::when_property_changed) that starts with the current
        /// values only when `notify_on_initial_value` is true
        /// (`WhenPropertyChanged(Expression<Func<TObject, TValue>>, bool)`).
        fn when_property_changed_with<P: crate::binding::IntoPropertyChain<T>>(
            &self,
            property_accessor: P,
            notify_on_initial_value: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::binding::PropertyValue<T, P::Value>>>
        where
            P::Value: 'static,
        {
            let chain = crate::binding::IntoPropertyChain::into_property_chain(property_accessor);
            crate::list::internal::merge_list_item_observables(
                self.change_sets(),
                std::rc::Rc::new(move |item: &T| crate::binding::observe_property(chain.clone(), item.clone(), notify_on_initial_value)),
            )
        }
    };
}
pub(crate) use when_property_changed_methods;
