//! `EnsureUniqueKeys` (`Cache/ObservableCacheEx.EnsureUniqueKeys.cs`).

macro_rules! ensure_unique_keys_methods {
    () => {
        /// The change sets with one change per key, the changes of a key within a change set
        /// combined; moves are left out (`EnsureUniqueKeys`).
        fn ensure_unique_keys(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::uniqueness_enforcer(self.change_sets())
        }
    };
}
pub(crate) use ensure_unique_keys_methods;
