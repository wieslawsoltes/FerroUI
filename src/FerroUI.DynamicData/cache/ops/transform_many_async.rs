//! `TransformManyAsync` (`Cache/ObservableCacheEx.TransformManyAsync.cs`).
//!
//! The children of each item of the source, given by a future, merged into one cache keyed by the
//! key of the children. Without a comparer, the first child of a key stays until it is removed;
//! with one, the child of a key is the least by the comparer. An error of a future ends the
//! sequence.
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's selector returns a `Task<..>`; here it returns
//! a future of `Result<.., ObservableError>`, which runs as a task of the thread when the children
//! are subscribed (docs/porting/reactive.md, section 5.8), and takes its arguments by value, so
//! that the future owns them. Upstream overloads the operator by what the future gives; here the
//! name says it: `transform_many_async*` for the items of an enumerable (a `Vec`),
//! `transform_many_async_observable_collection*` for an observable collection
//! (`ObservableCollection`, `ObservableCollectionExtended` or `ReadOnlyObservableCollection`,
//! upstream's `TCollection : INotifyCollectionChanged, IEnumerable<TDestination>`), and
//! `transform_many_async_observable_cache*` for an observable cache. The selector of the item and
//! its key is `_with_key`; `_with` takes it with the equality of the children (`PartialEq` when
//! `None`) and their comparer.

macro_rules! transform_many_async_methods {
    () => {
        /// The items that the future of `many_selector` gives for each item, keyed by
        /// `key_selector`
        /// (`TransformManyAsync(Func<TSource, Task<IEnumerable<TDestination>>>, Func<TDestination, TDestinationKey>, IEqualityComparer<TDestination>? = null, IComparer<TDestination>? = null)`).
        fn transform_many_async<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            Fut: std::future::Future<Output = Result<Vec<D>, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_async_with(move |item: T, _key: K| many_selector(item), key_selector, None, None)
        }

        /// [`transform_many_async`](Self::transform_many_async) with a selector of the item and its
        /// key (`TransformManyAsync(Func<TSource, TSourceKey, Task<IEnumerable<TDestination>>>, Func<TDestination, TDestinationKey>, ..)`).
        fn transform_many_async_with_key<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            Fut: std::future::Future<Output = Result<Vec<D>, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_async_with(many_selector, key_selector, None, None)
        }

        /// [`transform_many_async_with_key`](Self::transform_many_async_with_key) with the equality
        /// of the children (`PartialEq` when `None`) and their comparer
        /// (`TransformManyAsync(Func<TSource, TSourceKey, Task<IEnumerable<TDestination>>>, Func<TDestination, TDestinationKey>, IEqualityComparer<TDestination>?, IComparer<TDestination>?)`).
        fn transform_many_async_with<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            Fut: std::future::Future<Output = Result<Vec<D>, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::transform_many_async(
                self.change_sets(),
                crate::cache::internal::enumerable_change_set_transformer(many_selector, key_selector),
                equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
                comparer,
                None,
            )
        }

        /// The items of the observable collection that the future of `many_selector` gives for
        /// each item, keyed by `key_selector`, followed as the collections change
        /// (`TransformManyAsync<.., TCollection>(Func<TSource, Task<TCollection>>, Func<TDestination, TDestinationKey>, IEqualityComparer<TDestination>? = null, IComparer<TDestination>? = null)`).
        fn transform_many_async_observable_collection<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::binding::ObservableCollectionEx<D> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_async_observable_collection_with(move |item: T, _key: K| many_selector(item), key_selector, None, None)
        }

        /// [`transform_many_async_observable_collection`](Self::transform_many_async_observable_collection)
        /// with a selector of the item and its key
        /// (`TransformManyAsync<.., TCollection>(Func<TSource, TSourceKey, Task<TCollection>>, Func<TDestination, TDestinationKey>, ..)`).
        fn transform_many_async_observable_collection_with_key<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::binding::ObservableCollectionEx<D> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_async_observable_collection_with(many_selector, key_selector, None, None)
        }

        /// [`transform_many_async_observable_collection_with_key`](Self::transform_many_async_observable_collection_with_key)
        /// with the equality of the children (`PartialEq` when `None`) and their comparer
        /// (`TransformManyAsync<.., TCollection>(Func<TSource, TSourceKey, Task<TCollection>>, Func<TDestination, TDestinationKey>, IEqualityComparer<TDestination>?, IComparer<TDestination>?)`).
        fn transform_many_async_observable_collection_with<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::binding::ObservableCollectionEx<D> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::transform_many_async(
                self.change_sets(),
                crate::cache::internal::observable_collection_change_set_transformer(many_selector, key_selector),
                equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
                comparer,
                None,
            )
        }

        /// The items of the observable cache that the future of `many_selector` gives for each
        /// item, followed as the caches change
        /// (`TransformManyAsync(Func<TSource, Task<IObservableCache<TDestination, TDestinationKey>>>, IEqualityComparer<TDestination>? = null, IComparer<TDestination>? = null)`).
        fn transform_many_async_observable_cache<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::cache::IObservableCache<D, DK> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T) -> Fut + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_async_observable_cache_with(move |item: T, _key: K| many_selector(item), None, None)
        }

        /// [`transform_many_async_observable_cache`](Self::transform_many_async_observable_cache)
        /// with a selector of the item and its key
        /// (`TransformManyAsync(Func<TSource, TSourceKey, Task<IObservableCache<TDestination, TDestinationKey>>>, ..)`).
        fn transform_many_async_observable_cache_with_key<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::cache::IObservableCache<D, DK> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_async_observable_cache_with(many_selector, None, None)
        }

        /// [`transform_many_async_observable_cache_with_key`](Self::transform_many_async_observable_cache_with_key)
        /// with the equality of the children (`PartialEq` when `None`) and their comparer
        /// (`TransformManyAsync(Func<TSource, TSourceKey, Task<IObservableCache<TDestination, TDestinationKey>>>, IEqualityComparer<TDestination>?, IComparer<TDestination>?)`).
        fn transform_many_async_observable_cache_with<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::cache::IObservableCache<D, DK> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::transform_many_async(
                self.change_sets(),
                crate::cache::internal::observable_cache_change_set_transformer(many_selector),
                equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
                comparer,
                None,
            )
        }
    };
}
pub(crate) use transform_many_async_methods;
