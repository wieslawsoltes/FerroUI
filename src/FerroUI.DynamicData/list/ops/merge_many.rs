//! `MergeMany` (`List/ObservableListEx.MergeMany.cs`).

macro_rules! merge_many_methods {
    () => {
        /// The values of the sequence that `observable_selector` chooses for each item, merged; the
        /// sequence of an item is subscribed while the item is in the list. The merge completes
        /// when the source and every sequence have completed
        /// (`MergeMany(Func<T, IObservable<TDestination>>)`).
        fn merge_many<D: 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> {
            crate::list::internal::merge_many(self.change_sets(), std::rc::Rc::new(observable_selector))
        }
    };
}
pub(crate) use merge_many_methods;
