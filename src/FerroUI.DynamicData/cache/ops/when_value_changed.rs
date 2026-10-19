//! `WhenValueChanged` (`Cache/ObservableCacheEx.WhenValueChanged.cs`).

macro_rules! when_value_changed_methods {
    () => {
        /// The values of the property of `property_accessor` of every item, its current value
        /// first, merged; an item that a null step of the path leaves without a value produces
        /// nothing (`WhenValueChanged(Expression<Func<TObject, TValue>>, bool notifyOnInitialValue = true)`).
        fn when_value_changed<P: crate::binding::IntoPropertyChain<T>>(&self, property_accessor: P) -> std::rc::Rc<dyn ferroui_system::IObservable<P::Value>>
        where
            P::Value: 'static,
        {
            self.when_value_changed_with(property_accessor, true)
        }

        /// [`when_value_changed`](Self::when_value_changed) that starts with the current values
        /// only when `notify_on_initial_value` is true
        /// (`WhenValueChanged(Expression<Func<TObject, TValue>>, bool)`).
        fn when_value_changed_with<P: crate::binding::IntoPropertyChain<T>>(
            &self,
            property_accessor: P,
            notify_on_initial_value: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<P::Value>>
        where
            P::Value: 'static,
        {
            let chain = crate::binding::IntoPropertyChain::into_property_chain(property_accessor);
            crate::cache::internal::merge_item_observables(
                self.change_sets(),
                std::rc::Rc::new(move |item: &T, _: &K| crate::binding::when_changed_of(chain.clone(), item.clone(), notify_on_initial_value, None)),
            )
        }
    };
}
pub(crate) use when_value_changed_methods;
