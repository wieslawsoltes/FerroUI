//! `AddOrUpdate` (`Cache/ObservableCacheEx.AddOrUpdate.cs`): the edit conveniences of source
//! caches. The form on `IIntermediateCache` is `IntermediateCache::add_or_update`.

/// The forms on source caches, provided methods of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! add_or_update_source_methods {
    () => {
        /// Adds `item`, or replaces the item of its key (`AddOrUpdate(TObject)`).
        fn add_or_update(&self, item: T)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            let mut item = Some(item);
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| {
                if let Some(item) = item.take() {
                    updater.add_or_update(item);
                }
            });
        }

        /// [`add_or_update`](Self::add_or_update) that leaves an equal item, by `equality_comparer`,
        /// as it is (`AddOrUpdate(TObject, IEqualityComparer<TObject>)`).
        fn add_or_update_with_comparer(&self, item: T, equality_comparer: impl Fn(&T, &T) -> bool)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            let mut item = Some(item);
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| {
                if let Some(item) = item.take() {
                    updater.add_or_update_with(item, &equality_comparer);
                }
            });
        }

        /// Adds the items, or replaces the items of their keys (`AddOrUpdate(IEnumerable<TObject>)`).
        fn add_or_update_items(&self, items: impl IntoIterator<Item = T>)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            let mut items = Some(items.into_iter().collect::<Vec<T>>());
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| {
                if let Some(items) = items.take() {
                    updater.add_or_update_items(&mut items.into_iter());
                }
            });
        }

        /// [`add_or_update_items`](Self::add_or_update_items) that leaves the equal items, by
        /// `equality_comparer`, as they are (`AddOrUpdate(IEnumerable<TObject>,
        /// IEqualityComparer<TObject>)`).
        fn add_or_update_items_with_comparer(&self, items: impl IntoIterator<Item = T>, equality_comparer: impl Fn(&T, &T) -> bool)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            let mut items = Some(items.into_iter().collect::<Vec<T>>());
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| {
                if let Some(items) = items.take() {
                    updater.add_or_update_items_with(&mut items.into_iter(), &equality_comparer);
                }
            });
        }
    };
}
pub(crate) use add_or_update_source_methods;
