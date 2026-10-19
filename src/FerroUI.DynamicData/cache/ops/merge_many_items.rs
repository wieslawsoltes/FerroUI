//! `MergeManyItems` (`Cache/ObservableCacheEx.MergeManyItems.cs`).

macro_rules! merge_many_items_methods {
    () => {
        /// As [`merge_many`](Self::merge_many), with each value paired with the item whose sequence
        /// produced it (`MergeManyItems(Func<TObject, IObservable<TDestination>>)`).
        fn merge_many_items<D: 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::kernel::ItemWithValue<T, D>>> {
            crate::cache::internal::merge_many_items(self.change_sets(), std::rc::Rc::new(move |item: &T, _: &K| observable_selector(item)))
        }

        /// [`merge_many_items`](Self::merge_many_items) with a selector that takes the key
        /// (`MergeManyItems(Func<TObject, TKey, IObservable<TDestination>>)`).
        fn merge_many_items_with_key<D: 'static>(
            &self,
            observable_selector: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::kernel::ItemWithValue<T, D>>> {
            crate::cache::internal::merge_many_items(self.change_sets(), std::rc::Rc::new(observable_selector))
        }
    };
}
pub(crate) use merge_many_items_methods;
