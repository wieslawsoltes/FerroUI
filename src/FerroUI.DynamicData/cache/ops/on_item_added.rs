//! `OnItemAdded` (`Cache/ObservableCacheEx.OnItemAdded.cs`).

macro_rules! on_item_added_methods {
    () => {
        /// Runs `add_action` for each item added, before the change set is forwarded
        /// (`OnItemAdded(Action<TObject>)`).
        fn on_item_added(&self, add_action: impl Fn(&T) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::on_change_reason(self.change_sets(), crate::cache::ChangeReason::Add, move |item: &T, _: &K| add_action(item))
        }

        /// [`on_item_added`](Self::on_item_added) with the key (`OnItemAdded(Action<TObject, TKey>)`).
        fn on_item_added_with_key(&self, add_action: impl Fn(&T, &K) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::on_change_reason(self.change_sets(), crate::cache::ChangeReason::Add, add_action)
        }
    };
}
pub(crate) use on_item_added_methods;
