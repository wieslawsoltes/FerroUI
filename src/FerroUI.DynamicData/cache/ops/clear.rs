//! `Clear` (`Cache/ObservableCacheEx.Clear.cs`). The forms on `IIntermediateCache` and
//! `LockFreeObservableCache` are their `clear`.

/// The form on source caches, a provided method of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! clear_source_methods {
    () => {
        /// Removes every item (`Clear(ISourceCache)`).
        fn clear(&self)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            self.edit(&mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| updater.clear());
        }
    };
}
pub(crate) use clear_source_methods;
