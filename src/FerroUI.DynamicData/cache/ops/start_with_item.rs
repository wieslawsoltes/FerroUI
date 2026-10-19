//! `StartWithItem` (`Cache/ObservableCacheEx.StartWithItem.cs`).

macro_rules! start_with_item_methods {
    () => {
        /// A change set that adds `item` under `key`, then the change sets
        /// (`StartWithItem(TObject, TKey)`).
        fn start_with_item(&self, item: T, key: K) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            let change = crate::cache::Change::new(crate::cache::ChangeReason::Add, key, item);
            ferroui_reactive::ObservableExt::start_with(&self.change_sets(), [crate::cache::ChangeSet::from(vec![change])])
        }

        /// A change set that adds `item` under its own key, then the change sets
        /// (`StartWithItem(TObject)` for an item that is an `IKey<TKey>`).
        fn start_with_keyed_item(&self, item: T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: crate::cache::IKey<K>,
        {
            let key = crate::cache::IKey::key(&item);
            self.start_with_item(item, key)
        }
    };
}
pub(crate) use start_with_item_methods;
