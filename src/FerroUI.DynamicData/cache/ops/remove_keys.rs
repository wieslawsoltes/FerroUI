//! `RemoveKeys` (`Cache/ObservableCacheEx.RemoveKeys.cs`).

/// The form on source caches, a provided method of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! remove_keys_source_methods {
    () => {
        /// Removes the items of `keys` (`RemoveKeys(IEnumerable<TKey>)`, and
        /// `Remove(IEnumerable<TKey>)`).
        fn remove_keys(&self, keys: impl IntoIterator<Item = K>)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            let mut keys = Some(keys.into_iter().collect::<Vec<K>>());
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| {
                if let Some(keys) = keys.take() {
                    updater.remove_keys(&mut keys.into_iter());
                }
            });
        }
    };
}
pub(crate) use remove_keys_source_methods;
