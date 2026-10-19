//! `TransformMany` (`Cache/ObservableCacheEx.TransformMany.cs`).
//!
//! The items of each item of the source flattened into one cache, keyed by `key_selector`. An
//! update of an item of the source removes the items it no longer has, adds the new ones and
//! updates those of the same key, unless the item is the same reference.
//!
//! Deviation (DEVIATIONS.md, DynamicData): the reference of an item is the object its pointer
//! refers to (`TDestination: Deref`, an `Rc<..>` for the classes of upstream), as
//! `ignore_same_reference_update` does. The overloads for `ObservableCollection<T>` and
//! `ReadOnlyObservableCollection<T>` children are one method,
//! `transform_many_observable_collection`, generic over the collection
//! (`ObservableCollectionEx`, which `ObservableCollectionExtended<T>` has too).

macro_rules! transform_many_methods {
    () => {
        /// The items that `many_selector` gives for each item, keyed by `key_selector`
        /// (`TransformMany(Func<TSource, IEnumerable<TDestination>>, Func<TDestination, TDestinationKey>)`).
        /// A refresh of an item of the source takes its items again.
        fn transform_many<D: Clone + std::ops::Deref + 'static, DK: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            many_selector: impl Fn(&T) -> Vec<D> + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::transform_many(self.change_sets(), std::rc::Rc::new(many_selector), std::rc::Rc::new(key_selector))
        }

        /// The items of the cache that `many_selector` gives for each item, keyed by
        /// `key_selector`, followed as the caches change
        /// (`TransformMany(Func<TSource, IObservableCache<TDestination, TDestinationKey>>, Func<TDestination, TDestinationKey>)`).
        fn transform_many_observable_cache<D: Clone + std::ops::Deref + 'static, DK: Eq + std::hash::Hash + Clone + 'static, C: crate::cache::IObservableCache<D, DK> + 'static>(
            &self,
            many_selector: impl Fn(&T) -> C + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            let many_selector: std::rc::Rc<dyn Fn(&T) -> C> = std::rc::Rc::new(many_selector);
            let items_selector = many_selector.clone();
            let items: std::rc::Rc<dyn Fn(&T) -> Vec<D>> = std::rc::Rc::new(move |item: &T| crate::cache::IObservableCache::items(&items_selector(item)));
            crate::cache::internal::transform_many_with_child_changes(
                self.change_sets(),
                items,
                std::rc::Rc::new(key_selector),
                crate::cache::internal::observable_cache_child_changes(many_selector),
            )
        }

        /// The items of the observable collection that `many_selector` gives for each item, keyed
        /// by `key_selector`, followed as the collections change
        /// (`TransformMany(Func<TSource, ObservableCollection<TDestination>>, Func<TDestination, TDestinationKey>)`,
        /// `TransformMany(Func<TSource, ReadOnlyObservableCollection<TDestination>>, Func<TDestination, TDestinationKey>)`).
        fn transform_many_observable_collection<
            D: Clone + PartialEq + std::ops::Deref + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::binding::ObservableCollectionEx<D> + 'static,
        >(
            &self,
            many_selector: impl Fn(&T) -> C + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            let many_selector: std::rc::Rc<dyn Fn(&T) -> C> = std::rc::Rc::new(many_selector);
            let key_selector: std::rc::Rc<dyn Fn(&D) -> DK> = std::rc::Rc::new(key_selector);
            let items_selector = many_selector.clone();
            // The collection is the enumerable of its items, read when they are needed.
            let items: std::rc::Rc<dyn Fn(&T) -> Vec<D>> =
                std::rc::Rc::new(move |item: &T| crate::binding::ObservableCollectionEx::notifying_collection(&items_selector(item)).to_vec());
            crate::cache::internal::transform_many_with_child_changes(
                self.change_sets(),
                items,
                key_selector.clone(),
                crate::cache::internal::observable_collection_child_changes(many_selector, key_selector),
            )
        }
    };
}
pub(crate) use transform_many_methods;
