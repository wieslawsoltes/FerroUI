//! `DistinctValues` (`List/ObservableListEx.DistinctValues.cs`).

macro_rules! distinct_values_methods {
    () => {
        /// The distinct values that `value_selector` gives for the items, as the changes of a list
        /// (`DistinctValues(Func<TObject, TValue>)`).
        fn distinct_values<V: Eq + std::hash::Hash + Clone + 'static>(&self, value_selector: impl Fn(&T) -> V + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<V>>> {
            crate::list::internal::distinct(self.change_sets(), std::rc::Rc::new(value_selector))
        }
    };
}
pub(crate) use distinct_values_methods;
