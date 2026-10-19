//! `TransformManySafeAsync` (`Cache/ObservableCacheEx.TransformManySafeAsync.cs`).
//!
//! `TransformManyAsync` whose futures may fail: an error goes to `error_handler` with the item and
//! its key, and the item has no children; the sequence goes on. The forms are named as those of
//! `transform_many_async`, with the handler after the selectors.
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's selector returns a `Task<..>`; here it returns
//! a future of `Result<.., ObservableError>`, whose `Err` is upstream's exception
//! (docs/porting/reactive.md, section 5.8).

macro_rules! transform_many_safe_async_methods {
    () => {
        /// The items that the future of `many_selector` gives for each item, keyed by
        /// `key_selector`
        /// (`TransformManySafeAsync(Func<TSource, Task<IEnumerable<TDestination>>>, Func<TDestination, TDestinationKey>, IEqualityComparer<TDestination>? = null, IComparer<TDestination>? = null)`).
        fn transform_many_safe_async<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            Fut: std::future::Future<Output = Result<Vec<D>, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_safe_async_with(move |item: T, _key: K| many_selector(item), key_selector, error_handler, None, None)
        }

        /// [`transform_many_safe_async`](Self::transform_many_safe_async) with a selector of the item and its
        /// key (`TransformManySafeAsync(Func<TSource, TSourceKey, Task<IEnumerable<TDestination>>>, Func<TDestination, TDestinationKey>, ..)`).
        fn transform_many_safe_async_with_key<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            Fut: std::future::Future<Output = Result<Vec<D>, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_safe_async_with(many_selector, key_selector, error_handler, None, None)
        }

        /// [`transform_many_safe_async_with_key`](Self::transform_many_safe_async_with_key) with the equality
        /// of the children (`PartialEq` when `None`) and their comparer
        /// (`TransformManySafeAsync(Func<TSource, TSourceKey, Task<IEnumerable<TDestination>>>, Func<TDestination, TDestinationKey>, IEqualityComparer<TDestination>?, IComparer<TDestination>?)`).
        fn transform_many_safe_async_with<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            Fut: std::future::Future<Output = Result<Vec<D>, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::transform_many_async(
                self.change_sets(),
                crate::cache::internal::enumerable_change_set_transformer(many_selector, key_selector),
                equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
                comparer,
                Some(std::rc::Rc::new(error_handler)),
            )
        }

        /// The items of the observable collection that the future of `many_selector` gives for
        /// each item, keyed by `key_selector`, followed as the collections change
        /// (`TransformManySafeAsync<.., TCollection>(Func<TSource, Task<TCollection>>, Func<TDestination, TDestinationKey>, IEqualityComparer<TDestination>? = null, IComparer<TDestination>? = null)`).
        fn transform_many_safe_async_observable_collection<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::binding::ObservableCollectionEx<D> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_safe_async_observable_collection_with(move |item: T, _key: K| many_selector(item), key_selector, error_handler, None, None)
        }

        /// [`transform_many_safe_async_observable_collection`](Self::transform_many_safe_async_observable_collection)
        /// with a selector of the item and its key
        /// (`TransformManySafeAsync<.., TCollection>(Func<TSource, TSourceKey, Task<TCollection>>, Func<TDestination, TDestinationKey>, ..)`).
        fn transform_many_safe_async_observable_collection_with_key<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::binding::ObservableCollectionEx<D> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_safe_async_observable_collection_with(many_selector, key_selector, error_handler, None, None)
        }

        /// [`transform_many_safe_async_observable_collection_with_key`](Self::transform_many_safe_async_observable_collection_with_key)
        /// with the equality of the children (`PartialEq` when `None`) and their comparer
        /// (`TransformManySafeAsync<.., TCollection>(Func<TSource, TSourceKey, Task<TCollection>>, Func<TDestination, TDestinationKey>, IEqualityComparer<TDestination>?, IComparer<TDestination>?)`).
        fn transform_many_safe_async_observable_collection_with<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::binding::ObservableCollectionEx<D> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            key_selector: impl Fn(&D) -> DK + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::transform_many_async(
                self.change_sets(),
                crate::cache::internal::observable_collection_change_set_transformer(many_selector, key_selector),
                equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
                comparer,
                Some(std::rc::Rc::new(error_handler)),
            )
        }

        /// The items of the observable cache that the future of `many_selector` gives for each
        /// item, followed as the caches change
        /// (`TransformManySafeAsync(Func<TSource, Task<IObservableCache<TDestination, TDestinationKey>>>, IEqualityComparer<TDestination>? = null, IComparer<TDestination>? = null)`).
        fn transform_many_safe_async_observable_cache<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::cache::IObservableCache<D, DK> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_safe_async_observable_cache_with(move |item: T, _key: K| many_selector(item), error_handler, None, None)
        }

        /// [`transform_many_safe_async_observable_cache`](Self::transform_many_safe_async_observable_cache)
        /// with a selector of the item and its key
        /// (`TransformManySafeAsync(Func<TSource, TSourceKey, Task<IObservableCache<TDestination, TDestinationKey>>>, ..)`).
        fn transform_many_safe_async_observable_cache_with_key<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::cache::IObservableCache<D, DK> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            self.transform_many_safe_async_observable_cache_with(many_selector, error_handler, None, None)
        }

        /// [`transform_many_safe_async_observable_cache_with_key`](Self::transform_many_safe_async_observable_cache_with_key)
        /// with the equality of the children (`PartialEq` when `None`) and their comparer
        /// (`TransformManySafeAsync(Func<TSource, TSourceKey, Task<IObservableCache<TDestination, TDestinationKey>>>, IEqualityComparer<TDestination>?, IComparer<TDestination>?)`).
        fn transform_many_safe_async_observable_cache_with<
            D: Clone + PartialEq + 'static,
            DK: Eq + std::hash::Hash + Clone + 'static,
            C: crate::cache::IObservableCache<D, DK> + 'static,
            Fut: std::future::Future<Output = Result<C, ferroui_system::ObservableError>> + 'static,
        >(
            &self,
            many_selector: impl Fn(T, K) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&D, &D) -> std::cmp::Ordering>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<D, DK>>> {
            crate::cache::internal::transform_many_async(
                self.change_sets(),
                crate::cache::internal::observable_cache_change_set_transformer(many_selector),
                equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
                comparer,
                Some(std::rc::Rc::new(error_handler)),
            )
        }
    };
}
pub(crate) use transform_many_safe_async_methods;
