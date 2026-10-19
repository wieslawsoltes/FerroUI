//! `DistinctValues` (`Cache/ObservableCacheEx.DistinctValues.cs`).

macro_rules! distinct_values_methods {
    () => {
        /// The distinct values that `value_selector` gives for the items, as the changes of a cache
        /// keyed by the values (`DistinctValues(Func<TObject, TValue>)`).
        fn distinct_values<V: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            value_selector: impl Fn(&T) -> V + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::DistinctChangeSet<V>>> {
            crate::cache::internal::distinct_calculator(self.change_sets(), std::rc::Rc::new(value_selector))
        }
    };
}
pub(crate) use distinct_values_methods;
