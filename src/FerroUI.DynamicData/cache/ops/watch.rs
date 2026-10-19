//! `Watch` (`Cache/ObservableCacheEx.Watch.cs`).

macro_rules! watch_methods {
    () => {
        /// The changes of the item of `key` (`Watch(IObservable<IChangeSet>, TKey)`).
        fn watch(&self, key: K) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::Change<T, K>>> {
            let changes = ferroui_reactive::ObservableExt::select_many_iter(&self.change_sets(), |updates: crate::cache::ChangeSet<T, K>| updates.to_vec());
            ferroui_reactive::ObservableExt::where_(&changes, move |update: &crate::cache::Change<T, K>| *update.key() == key)
        }
    };
}
pub(crate) use watch_methods;
