//! `TransformSafeAsync` (`Cache/ObservableCacheEx.TransformSafeAsync.cs`).
//!
//! `TransformAsync` whose factory may fail: an error of a future goes to `error_handler` with the
//! item and its key, the item is left out, and the sequence goes on. The forms are named as those
//! of `transform_async`.
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's factory returns a `Task<TDestination>`; here
//! it returns a future of `Result<TDestination, ObservableError>`, whose `Err` is upstream's
//! exception (docs/porting/reactive.md, section 5.8).

macro_rules! transform_safe_async_methods {
    () => {
        /// Each item in the form the future of `transform_factory` gives it: an addition or an
        /// update awaits the factory, a removal removes the item, a refresh is forwarded
        /// (`TransformSafeAsync(Func<TSource, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_safe_async<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, _key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current)),
                Some(std::rc::Rc::new(error_handler)),
                None,
                None,
                false,
            )
        }

        /// [`transform_safe_async`](Self::transform_safe_async) with a factory of the item and its key
        /// (`TransformSafeAsync(Func<TSource, TKey, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_safe_async_with_key<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, K) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current, key)),
                Some(std::rc::Rc::new(error_handler)),
                None,
                None,
                false,
            )
        }

        /// [`transform_safe_async`](Self::transform_safe_async) with a factory of the item, the item it
        /// replaces (for an update) and the key
        /// (`TransformSafeAsync(Func<TSource, Optional<TSource>, TKey, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_safe_async_with_previous<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| {
                    transform_factory(current, previous, key)
                }),
                Some(std::rc::Rc::new(error_handler)),
                None,
                None,
                false,
            )
        }

        /// [`transform_safe_async_with_previous`](Self::transform_safe_async_with_previous) whose factory
        /// also takes the token that the disposal of the subscription cancels
        /// (`TransformSafeAsync(Func<TSource, Optional<TSource>, TKey, CancellationToken, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_safe_async_with_token<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K, ferroui_system::threading::CancellationToken) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(transform_factory),
                Some(std::rc::Rc::new(error_handler)),
                None,
                None,
                false,
            )
        }

        /// [`transform_safe_async`](Self::transform_safe_async) that also transforms again, as updates, the
        /// items that each predicate of `force_transform` accepts
        /// (`TransformSafeAsync(Func<TSource, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_safe_async_forced<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, _key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current)),
                Some(std::rc::Rc::new(error_handler)),
                Some(force_transform),
                None,
                false,
            )
        }

        /// [`transform_safe_async_with_key`](Self::transform_safe_async_with_key) with the predicates of
        /// [`transform_safe_async_forced`](Self::transform_safe_async_forced)
        /// (`TransformSafeAsync(Func<TSource, TKey, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_safe_async_with_key_forced<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, K) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current, key)),
                Some(std::rc::Rc::new(error_handler)),
                Some(force_transform),
                None,
                false,
            )
        }

        /// [`transform_safe_async_with_previous`](Self::transform_safe_async_with_previous) with the
        /// predicates of [`transform_safe_async_forced`](Self::transform_safe_async_forced)
        /// (`TransformSafeAsync(Func<TSource, Optional<TSource>, TKey, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_safe_async_with_previous_forced<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| {
                    transform_factory(current, previous, key)
                }),
                Some(std::rc::Rc::new(error_handler)),
                Some(force_transform),
                None,
                false,
            )
        }

        /// [`transform_safe_async_with_token`](Self::transform_safe_async_with_token) with the predicates of
        /// [`transform_safe_async_forced`](Self::transform_safe_async_forced)
        /// (`TransformSafeAsync(Func<TSource, Optional<TSource>, TKey, CancellationToken, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_safe_async_with_token_forced<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K, ferroui_system::threading::CancellationToken) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(transform_factory),
                Some(std::rc::Rc::new(error_handler)),
                Some(force_transform),
                None,
                false,
            )
        }

        /// [`transform_safe_async`](Self::transform_safe_async) with the greatest number of transforms in
        /// flight and the handling of refreshes of `options`
        /// (`TransformSafeAsync(Func<TSource, Task<TDestination>>, TransformAsyncOptions)`).
        fn transform_safe_async_with_options<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            options: crate::cache::TransformAsyncOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, _key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current)),
                Some(std::rc::Rc::new(error_handler)),
                None,
                options.maximum_concurrency,
                options.transform_on_refresh,
            )
        }

        /// [`transform_safe_async_with_key`](Self::transform_safe_async_with_key) with `options`
        /// (`TransformSafeAsync(Func<TSource, TKey, Task<TDestination>>, TransformAsyncOptions)`).
        fn transform_safe_async_with_key_and_options<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, K) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            options: crate::cache::TransformAsyncOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current, key)),
                Some(std::rc::Rc::new(error_handler)),
                None,
                options.maximum_concurrency,
                options.transform_on_refresh,
            )
        }

        /// [`transform_safe_async_with_previous`](Self::transform_safe_async_with_previous) with `options`
        /// (`TransformSafeAsync(Func<TSource, Optional<TSource>, TKey, Task<TDestination>>, TransformAsyncOptions)`).
        fn transform_safe_async_with_previous_and_options<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            options: crate::cache::TransformAsyncOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| {
                    transform_factory(current, previous, key)
                }),
                Some(std::rc::Rc::new(error_handler)),
                None,
                options.maximum_concurrency,
                options.transform_on_refresh,
            )
        }

        /// [`transform_safe_async_with_token`](Self::transform_safe_async_with_token) with `options`
        /// (`TransformSafeAsync(Func<TSource, Optional<TSource>, TKey, CancellationToken, Task<TDestination>>, TransformAsyncOptions)`).
        fn transform_safe_async_with_token_and_options<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K, ferroui_system::threading::CancellationToken) -> Fut + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            options: crate::cache::TransformAsyncOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(transform_factory),
                Some(std::rc::Rc::new(error_handler)),
                None,
                options.maximum_concurrency,
                options.transform_on_refresh,
            )
        }
    };
}
pub(crate) use transform_safe_async_methods;
