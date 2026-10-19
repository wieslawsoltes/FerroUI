//! `RemoveKey` (`Cache/ObservableCacheEx.RemoveKey.cs`).

macro_rules! remove_key_methods {
    () => {
        /// The changes as changes of a list, without their keys: an update becomes a removal and an
        /// addition, and a refresh a replacement of the item by itself (`RemoveKey()`).
        fn remove_key(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>
        where
            T: PartialEq,
        {
            ferroui_reactive::ObservableExt::select(&self.change_sets(), |changes: crate::cache::ChangeSet<T, K>| {
                crate::list::ChangeSet::from(crate::cache::internal::remove_key_changes(&changes, None))
            })
        }
    };
}
pub(crate) use remove_key_methods;

/// The form on source caches, a provided method of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! remove_key_source_methods {
    () => {
        /// Removes the item of `key` (`RemoveKey(TKey)`, and `Remove(TKey)`).
        fn remove_key(&self, key: K)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| updater.remove_key(&key));
        }
    };
}
pub(crate) use remove_key_source_methods;
