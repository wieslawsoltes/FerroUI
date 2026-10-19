//! `MergeManyChangeSets` (`List/ObservableListEx.MergeManyChangeSets.cs`): the change sets that a
//! selector chooses for each item of a list, merged into those of one list or one cache.
//!
//! Upstream overloads the operator by the kind of the change sets the selector returns; here the
//! name says it (Deviation (DEVIATIONS.md, DynamicData)): `merge_many_change_sets*` for change sets
//! of lists, `merge_many_cache_change_sets*` for change sets of caches.

macro_rules! merge_many_change_sets_methods {
    () => {
        /// The change sets of the list that `observable_selector` chooses for each item, merged
        /// (`MergeManyChangeSets(Func<TObject, IObservable<IChangeSet<TDestination>>>,
        /// IEqualityComparer<TDestination>? = null)`).
        fn merge_many_change_sets<D: Clone + PartialEq + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> {
            crate::list::internal::merge_many_list_change_sets(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                crate::cache::internal::default_equality_comparer(),
            )
        }

        /// [`merge_many_change_sets`](Self::merge_many_change_sets) with the equality of the
        /// children, which the copies of the child lists use
        /// (`MergeManyChangeSets(.., IEqualityComparer<TDestination>)`).
        fn merge_many_change_sets_with_comparer<D: Clone + PartialEq + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> + 'static,
            equality_comparer: impl Fn(&D, &D) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> {
            crate::list::internal::merge_many_list_change_sets(self.change_sets(), std::rc::Rc::new(observable_selector), std::rc::Rc::new(equality_comparer))
        }

        /// The change sets of the cache that `observable_selector` chooses for each item, merged
        /// into those of one cache (`MergeManyChangeSets(Func<TObject, IObservable<IChangeSet<
        /// TDestination, TDestinationKey>>>, IEqualityComparer<TDestination>? = null,
        /// IComparer<TDestination>? = null)`).
        fn merge_many_cache_change_sets<D: Clone + PartialEq + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::list::internal::merge_many_cache_change_sets(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                crate::cache::internal::default_equality_comparer(),
                None,
            )
        }

        /// [`merge_many_cache_change_sets`](Self::merge_many_cache_change_sets) with the equality of
        /// the children (`MergeManyChangeSets(.., IEqualityComparer<TDestination>)`).
        fn merge_many_cache_change_sets_with_comparer<D: Clone + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
            equality_comparer: impl Fn(&D, &D) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::list::internal::merge_many_cache_change_sets(self.change_sets(), std::rc::Rc::new(observable_selector), std::rc::Rc::new(equality_comparer), None)
        }

        /// [`merge_many_cache_change_sets`](Self::merge_many_cache_change_sets) where the value of a
        /// key is the least child by `comparer` (`MergeManyChangeSets(.., IComparer<TDestination>)`).
        fn merge_many_cache_change_sets_ordered_by<D: Clone + PartialEq + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
            comparer: std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::list::internal::merge_many_cache_change_sets(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                crate::cache::internal::default_equality_comparer(),
                Some(comparer),
            )
        }

        /// [`merge_many_cache_change_sets`](Self::merge_many_cache_change_sets) with the equality of
        /// the children (`PartialEq` when `None`) and their comparer
        /// (`MergeManyChangeSets(.., IEqualityComparer<TDestination>?, IComparer<TDestination>?)`).
        fn merge_many_cache_change_sets_with<D: Clone + PartialEq + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::list::internal::merge_many_cache_change_sets(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
                comparer,
            )
        }
    };
}
pub(crate) use merge_many_change_sets_methods;
