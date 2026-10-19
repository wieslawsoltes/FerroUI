//! `Sort` (`Cache/ObservableCacheEx.Sort.cs`), obsolete upstream in favour of `SortAndBind`.
//!
//! Each upstream overload has defaults (`SortOptimisations.None`, a reset threshold of 100); the
//! port has a short form with the defaults and a `_with` form with every argument. The forms with
//! an observable comparer are `sort_dynamic*`, as the other operators that take an observable
//! argument (`filter_dynamic`).

/// The reset threshold of upstream's sorts (`ObservableCacheEx.DefaultSortResetThreshold`).
pub const DEFAULT_SORT_RESET_THRESHOLD: usize = 100;

macro_rules! sort_methods {
    () => {
        /// The items, sorted by `comparer`: each change set has the indexes of its changes and
        /// the sorted items (`Sort(IComparer<TObject>)` with the default optimisations and reset
        /// threshold).
        fn sort(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::SortedChangeSet<T, K>>> {
            crate::cache::internal::sort(
                self.change_sets(),
                Some(comparer),
                crate::cache::SortOptimisations::NONE,
                None,
                None,
                crate::cache::DEFAULT_SORT_RESET_THRESHOLD,
            )
        }

        /// [`sort`](Self::sort) with `sort_optimisations` and `reset_threshold`, the number of
        /// changes from which the sorted items are rebuilt rather than edited (0 never rebuilds)
        /// (`Sort(IComparer<TObject>, SortOptimisations, int resetThreshold)`).
        fn sort_with(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            sort_optimisations: crate::cache::SortOptimisations,
            reset_threshold: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::SortedChangeSet<T, K>>> {
            crate::cache::internal::sort(self.change_sets(), Some(comparer), sort_optimisations, None, None, reset_threshold)
        }

        /// The items, sorted by the latest comparer of `comparer_observable`; nothing is produced
        /// before the first comparer (`Sort(IObservable<IComparer<TObject>>)` with the default
        /// optimisations and reset threshold).
        fn sort_dynamic(
            &self,
            comparer_observable: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::SortedChangeSet<T, K>>> {
            crate::cache::internal::sort(
                self.change_sets(),
                None,
                crate::cache::SortOptimisations::NONE,
                Some(comparer_observable),
                None,
                crate::cache::DEFAULT_SORT_RESET_THRESHOLD,
            )
        }

        /// [`sort_dynamic`](Self::sort_dynamic) with `sort_optimisations` and `reset_threshold`
        /// (`Sort(IObservable<IComparer<TObject>>, SortOptimisations, int resetThreshold)`).
        fn sort_dynamic_with(
            &self,
            comparer_observable: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
            sort_optimisations: crate::cache::SortOptimisations,
            reset_threshold: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::SortedChangeSet<T, K>>> {
            crate::cache::internal::sort(self.change_sets(), None, sort_optimisations, Some(comparer_observable), None, reset_threshold)
        }

        /// [`sort_dynamic_with`](Self::sort_dynamic_with) that also sorts again, with moves,
        /// whenever `resorter` notifies (`Sort(IObservable<IComparer<TObject>>, IObservable<Unit>,
        /// SortOptimisations, int resetThreshold)`).
        fn sort_dynamic_with_resorter(
            &self,
            comparer_observable: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>>,
            resorter: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
            sort_optimisations: crate::cache::SortOptimisations,
            reset_threshold: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::SortedChangeSet<T, K>>> {
            crate::cache::internal::sort(self.change_sets(), None, sort_optimisations, Some(comparer_observable), Some(resorter), reset_threshold)
        }

        /// [`sort_with`](Self::sort_with) that also sorts again, with moves, whenever `resorter`
        /// notifies (`Sort(IComparer<TObject>, IObservable<Unit>, SortOptimisations, int
        /// resetThreshold)`).
        fn sort_with_resorter(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            resorter: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
            sort_optimisations: crate::cache::SortOptimisations,
            reset_threshold: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::SortedChangeSet<T, K>>> {
            crate::cache::internal::sort(self.change_sets(), Some(comparer), sort_optimisations, None, Some(resorter), reset_threshold)
        }
    };
}
pub(crate) use sort_methods;
