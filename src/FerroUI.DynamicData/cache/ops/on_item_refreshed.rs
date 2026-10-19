//! `OnItemRefreshed` (`Cache/ObservableCacheEx.OnItemRefreshed.cs`).

macro_rules! on_item_refreshed_methods {
    () => {
        /// Runs `refresh_action` for each item refreshed, before the change set is forwarded
        /// (`OnItemRefreshed(Action<TObject>)`).
        fn on_item_refreshed(&self, refresh_action: impl Fn(&T) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::on_change_reason(self.change_sets(), crate::cache::ChangeReason::Refresh, move |item: &T, _: &K| refresh_action(item))
        }

        /// [`on_item_refreshed`](Self::on_item_refreshed) with the key
        /// (`OnItemRefreshed(Action<TObject, TKey>)`).
        fn on_item_refreshed_with_key(&self, refresh_action: impl Fn(&T, &K) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::on_change_reason(self.change_sets(), crate::cache::ChangeReason::Refresh, refresh_action)
        }
    };
}
pub(crate) use on_item_refreshed_methods;
