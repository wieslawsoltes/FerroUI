//! `MergeManyChangeSets` (`Cache/ObservableCacheEx.MergeManyChangeSets.cs`): the change sets that
//! a selector chooses for each item, merged into those of one cache or one list.
//!
//! Upstream overloads the operator by the kind of the change sets the selector returns; here the
//! name says it (Deviation (DEVIATIONS.md, DynamicData)): `merge_many_change_sets*` for change sets
//! of caches, `merge_many_list_change_sets*` for change sets of lists. The forms that take the key
//! in the selector are `_with_key`, or the full `_with` forms. Without a comparer, the first value
//! of a key stays until it is removed; with one (`_ordered_by`), the value of a key is the least
//! among the children; with a comparer of the parents (`_with_source_comparer`), the children of
//! the least parent win first. The default equality of the children is `PartialEq`.

macro_rules! merge_many_change_sets_methods {
    () => {
        /// The change sets of the cache that `observable_selector` chooses for each item, merged
        /// (`MergeManyChangeSets(Func<TObject, IObservable<IChangeSet<TDestination,
        /// TDestinationKey>>>, IEqualityComparer<TDestination>? = null, IComparer<TDestination>? =
        /// null)`).
        fn merge_many_change_sets<D: Clone + PartialEq + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::merge_many_cache_change_sets(
                self.change_sets(),
                std::rc::Rc::new(move |item: &T, _: &K| observable_selector(item)),
                crate::cache::internal::default_equality_comparer(),
                None,
            )
        }

        /// [`merge_many_change_sets`](Self::merge_many_change_sets) with a selector that takes the
        /// key (`MergeManyChangeSets(Func<TObject, TKey, IObservable<IChangeSet<TDestination,
        /// TDestinationKey>>>, ..)`).
        fn merge_many_change_sets_with_key<D: Clone + PartialEq + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::merge_many_cache_change_sets(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                crate::cache::internal::default_equality_comparer(),
                None,
            )
        }

        /// [`merge_many_change_sets`](Self::merge_many_change_sets) with the equality of the
        /// children (`MergeManyChangeSets(.., IEqualityComparer<TDestination>)`).
        fn merge_many_change_sets_with_comparer<D: Clone + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
            equality_comparer: impl Fn(&D, &D) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::merge_many_cache_change_sets(
                self.change_sets(),
                std::rc::Rc::new(move |item: &T, _: &K| observable_selector(item)),
                std::rc::Rc::new(equality_comparer),
                None,
            )
        }

        /// [`merge_many_change_sets`](Self::merge_many_change_sets) where the value of a key is the
        /// least child by `comparer` (`MergeManyChangeSets(.., IComparer<TDestination>)`).
        fn merge_many_change_sets_ordered_by<D: Clone + PartialEq + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
            comparer: std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::merge_many_cache_change_sets(
                self.change_sets(),
                std::rc::Rc::new(move |item: &T, _: &K| observable_selector(item)),
                crate::cache::internal::default_equality_comparer(),
                Some(comparer),
            )
        }

        /// [`merge_many_change_sets`](Self::merge_many_change_sets) with every argument: a selector
        /// that takes the key, the equality of the children (`PartialEq` when `None`) and their
        /// comparer (`MergeManyChangeSets(Func<TObject, TKey, ..>, IEqualityComparer<TDestination>?,
        /// IComparer<TDestination>?)`).
        fn merge_many_change_sets_with<D: Clone + PartialEq + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::merge_many_cache_change_sets(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
                comparer,
            )
        }

        /// The change sets of the cache chosen for each item, merged, where the children of the
        /// least parent by `source_comparer` win; a refresh of a parent chooses its children again
        /// (`MergeManyChangeSets(Func<TObject, IObservable<..>>, IComparer<TObject> sourceComparer,
        /// IEqualityComparer<TDestination>? = null, IComparer<TDestination>? = null)`).
        fn merge_many_change_sets_with_source_comparer<D: Clone + PartialEq + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
            source_comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>>
        where
            T: PartialEq,
        {
            crate::cache::internal::merge_many_cache_change_sets_source_compare(
                self.change_sets(),
                std::rc::Rc::new(move |item: &T, _: &K| observable_selector(item)),
                source_comparer,
                crate::cache::internal::entry_equality(),
                None,
                true,
            )
        }

        /// [`merge_many_change_sets_with_source_comparer`](Self::merge_many_change_sets_with_source_comparer)
        /// with every argument: a selector that takes the key, whether a refresh of a parent chooses
        /// its children again, the equality of the children (that of the parents and the children
        /// when `None`) and the comparer of the children (`MergeManyChangeSets(Func<TObject, TKey,
        /// ..>, IComparer<TObject>, bool resortOnSourceRefresh, IEqualityComparer<TDestination>?,
        /// IComparer<TDestination>?)`).
        fn merge_many_change_sets_with_source_comparer_with<D: Clone + PartialEq + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            observable_selector: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> + 'static,
            source_comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
            resort_on_source_refresh: bool,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
            child_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>>
        where
            T: PartialEq,
        {
            let equality_comparer = match equality_comparer {
                Some(equality_comparer) => crate::cache::internal::child_equality(equality_comparer),
                None => crate::cache::internal::entry_equality(),
            };
            crate::cache::internal::merge_many_cache_change_sets_source_compare(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                source_comparer,
                equality_comparer,
                child_comparer,
                resort_on_source_refresh,
            )
        }

        /// The change sets of the list that `observable_selector` chooses for each item, merged into
        /// those of one list (`MergeManyChangeSets(Func<TObject, IObservable<IChangeSet<
        /// TDestination>>>, IEqualityComparer<TDestination>? = null)`).
        fn merge_many_list_change_sets<D: Clone + PartialEq + 'static>(
            &self,
            observable_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> {
            crate::cache::internal::merge_many_list_change_sets(
                self.change_sets(),
                std::rc::Rc::new(move |item: &T, _: &K| observable_selector(item)),
                crate::cache::internal::default_equality_comparer(),
            )
        }

        /// [`merge_many_list_change_sets`](Self::merge_many_list_change_sets) with a selector that
        /// takes the key (`MergeManyChangeSets(Func<TObject, TKey, IObservable<IChangeSet<
        /// TDestination>>>, ..)`).
        fn merge_many_list_change_sets_with_key<D: Clone + PartialEq + 'static>(
            &self,
            observable_selector: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> {
            crate::cache::internal::merge_many_list_change_sets(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                crate::cache::internal::default_equality_comparer(),
            )
        }

        /// [`merge_many_list_change_sets_with_key`](Self::merge_many_list_change_sets_with_key) with
        /// the equality of the children, which the copies of the child lists use
        /// (`MergeManyChangeSets(Func<TObject, TKey, ..>, IEqualityComparer<TDestination>)`).
        fn merge_many_list_change_sets_with<D: Clone + PartialEq + 'static>(
            &self,
            observable_selector: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> + 'static,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<D>>> {
            crate::cache::internal::merge_many_list_change_sets(
                self.change_sets(),
                std::rc::Rc::new(observable_selector),
                equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
            )
        }
    };
}
pub(crate) use merge_many_change_sets_methods;
