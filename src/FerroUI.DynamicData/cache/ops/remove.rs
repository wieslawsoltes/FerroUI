//! `Remove` (`Cache/ObservableCacheEx.Remove.cs`). The forms on `IIntermediateCache` are
//! `IntermediateCache::remove_key` and `remove_keys`.

/// The forms on source caches, provided methods of [`SourceCacheEx`](crate::cache::SourceCacheEx).
/// `Remove(TKey)` and `Remove(IEnumerable<TKey>)` are [`remove_key`](crate::cache::SourceCacheEx::remove_key)
/// and [`remove_keys`](crate::cache::SourceCacheEx::remove_keys), as upstream's `RemoveKey` and
/// `RemoveKeys` are the same.
macro_rules! remove_source_methods {
    () => {
        /// Removes the item of the key of `item` (`Remove(TObject)`).
        fn remove(&self, item: &T)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| updater.remove_item(item));
        }

        /// Removes the items of the keys of `items` (`Remove(IEnumerable<TObject>)`).
        fn remove_items(&self, items: impl IntoIterator<Item = T>)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            let mut items = Some(items.into_iter().collect::<Vec<T>>());
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| {
                if let Some(items) = items.take() {
                    updater.remove_items(&mut items.into_iter());
                }
            });
        }
    };
}
pub(crate) use remove_source_methods;
