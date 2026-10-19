//! `TrueForAny` (`Cache/ObservableCacheEx.TrueForAny.cs`).

macro_rules! true_for_any_methods {
    () => {
        /// Whether `equality_condition` holds for the latest value of the observable that
        /// `observable_selector` gives for some item, each time the result changes
        /// (`TrueForAny(Func<TObject, IObservable<TValue>>, Func<TValue, bool>)`).
        fn true_for_any<V: Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<V>> + 'static,
            equality_condition: impl Fn(&V) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::cache::internal::true_for(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                std::rc::Rc::new(move |items: &[std::rc::Rc<crate::cache::internal::ObservableWithValue<T, V>>]| {
                    items.iter().any(|o| o.latest_value().is_some_and(|value| equality_condition(&value)))
                }),
            )
        }

        /// [`true_for_any`](Self::true_for_any) with a condition of the item and its value
        /// (`TrueForAny(Func<TObject, IObservable<TValue>>, Func<TObject, TValue, bool>)`).
        fn true_for_any_with_item<V: Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<V>> + 'static,
            equality_condition: impl Fn(&T, &V) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::cache::internal::true_for(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                std::rc::Rc::new(move |items: &[std::rc::Rc<crate::cache::internal::ObservableWithValue<T, V>>]| {
                    items.iter().any(|o| o.latest_value().is_some_and(|value| equality_condition(o.item(), &value)))
                }),
            )
        }
    };
}
pub(crate) use true_for_any_methods;
