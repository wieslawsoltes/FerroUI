//! `OnItemUpdated` (`Cache/ObservableCacheEx.OnItemUpdated.cs`).

macro_rules! on_item_updated_methods {
    () => {
        /// Runs `update_action` with the new item and the one it replaces, for each update, before
        /// the change set is forwarded (`OnItemUpdated(Action<TObject, TObject>)`).
        fn on_item_updated(&self, update_action: impl Fn(&T, &T) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::on_change_action(
                self.change_sets(),
                |change: &crate::cache::Change<T, K>| change.reason() == crate::cache::ChangeReason::Update,
                move |change: &crate::cache::Change<T, K>| update_action(change.current(), change.previous().expect("an update has a previous item")),
            )
        }

        /// [`on_item_updated`](Self::on_item_updated) with the key
        /// (`OnItemUpdated(Action<TObject, TObject, TKey>)`).
        fn on_item_updated_with_key(&self, update_action: impl Fn(&T, &T, &K) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::on_change_action(
                self.change_sets(),
                |change: &crate::cache::Change<T, K>| change.reason() == crate::cache::ChangeReason::Update,
                move |change: &crate::cache::Change<T, K>| update_action(change.current(), change.previous().expect("an update has a previous item"), change.key()),
            )
        }
    };
}
pub(crate) use on_item_updated_methods;
