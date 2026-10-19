//! `Refresh` (`Cache/ObservableCacheEx.Refresh.cs`).

/// The forms on source caches, provided methods of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! refresh_source_methods {
    () => {
        /// Signals that `item` has changed (`Refresh(TObject)`).
        fn refresh(&self, item: &T)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| updater.refresh_item(item));
        }

        /// Signals that the items have changed (`Refresh(IEnumerable<TObject>)`).
        fn refresh_items(&self, items: impl IntoIterator<Item = T>)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            let mut items = Some(items.into_iter().collect::<Vec<T>>());
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| {
                if let Some(items) = items.take() {
                    updater.refresh_items(&mut items.into_iter());
                }
            });
        }

        /// Signals that every item has changed (`Refresh()`).
        fn refresh_all(&self)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| updater.refresh_all());
        }
    };
}
pub(crate) use refresh_source_methods;
