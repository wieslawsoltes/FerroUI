//! `SortBy` (`Cache/ObservableCacheEx.SortBy.cs`).

macro_rules! sort_by_methods {
    () => {
        /// The items, sorted by the value that `expression` reads, ascending
        /// (`SortBy(Func<TObject, IComparable>)` with the default direction, optimisations and
        /// reset threshold).
        fn sort_by<V: PartialOrd + 'static>(
            &self,
            expression: impl Fn(&T) -> V + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::SortedChangeSet<T, K>>> {
            self.sort_by_with(
                expression,
                crate::binding::SortDirection::Ascending,
                crate::cache::SortOptimisations::NONE,
                crate::cache::DEFAULT_SORT_RESET_THRESHOLD,
            )
        }

        /// The items, sorted by the value that `expression` reads, in `sort_order`
        /// (`SortBy(Func<TObject, IComparable>, SortDirection, SortOptimisations, int)`).
        fn sort_by_with<V: PartialOrd + 'static>(
            &self,
            expression: impl Fn(&T) -> V + 'static,
            sort_order: crate::binding::SortDirection,
            sort_optimisations: crate::cache::SortOptimisations,
            reset_threshold: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::SortedChangeSet<T, K>>> {
            let comparer = match sort_order {
                crate::binding::SortDirection::Descending => crate::binding::SortExpressionComparer::<T>::descending(expression),
                crate::binding::SortDirection::Ascending => crate::binding::SortExpressionComparer::<T>::ascending(expression),
            };
            self.sort_with(comparer.comparer(), sort_optimisations, reset_threshold)
        }
    };
}
pub(crate) use sort_by_methods;
