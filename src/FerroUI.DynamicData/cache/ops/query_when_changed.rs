//! `QueryWhenChanged` (`Cache/ObservableCacheEx.QueryWhenChanged.cs`).

macro_rules! query_when_changed_methods {
    () => {
        /// A snapshot of the items to query after each change set (`QueryWhenChanged()`).
        fn query_when_changed(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::cache::IQuery<T, K>>>> {
            crate::cache::internal::query_when_changed(self.change_sets())
        }

        /// What `result_selector` makes of a snapshot of the items after each change set
        /// (`QueryWhenChanged(Func<IQuery<TObject, TKey>, TDestination>)`).
        fn query_when_changed_with<R: 'static>(
            &self,
            result_selector: impl Fn(std::rc::Rc<dyn crate::cache::IQuery<T, K>>) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            ferroui_reactive::ObservableExt::select(&self.query_when_changed(), result_selector)
        }

        /// A snapshot of the items after each change set and each value of the observable that
        /// `item_changed_trigger` gives for an item (`QueryWhenChanged(Func<TObject,
        /// IObservable<TValue>>)`).
        fn query_when_changed_with_trigger<V: 'static>(
            &self,
            item_changed_trigger: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<V>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::cache::IQuery<T, K>>>> {
            crate::cache::internal::query_when_changed_with_trigger(self.change_sets(), std::rc::Rc::new(item_changed_trigger))
        }
    };
}
pub(crate) use query_when_changed_methods;
