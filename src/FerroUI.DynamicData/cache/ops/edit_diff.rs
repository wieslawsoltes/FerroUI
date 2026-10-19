//! `EditDiff` (`Cache/ObservableCacheEx.EditDiff.cs`).
//!
//! The form on sequences of collections is a method of
//! [`ObservableSequencesEx`](crate::ObservableSequencesEx) and the one on sequences of options a
//! method of [`OptionObservableExtensions`](crate::kernel::OptionObservableExtensions).

/// The form on source caches, a provided method of [`SourceCacheEx`](crate::cache::SourceCacheEx).
macro_rules! edit_diff_source_methods {
    () => {
        /// Makes the cache hold `all_items` with the fewest changes: the items whose keys are gone
        /// are removed, the new keys added, and the items that `are_items_equal` says differ updated
        /// (`EditDiff(IEnumerable<TObject>, Func<TObject, TObject, bool>)` and
        /// `EditDiff(IEnumerable<TObject>, IEqualityComparer<TObject>)`).
        fn edit_diff(&self, all_items: impl IntoIterator<Item = T>, are_items_equal: impl Fn(&T, &T) -> bool)
        where
            Self: crate::cache::ISourceCache<T, K>,
        {
            crate::cache::internal::edit_diff(self, all_items.into_iter().collect(), &are_items_equal);
        }
    };
}
pub(crate) use edit_diff_source_methods;
