//! `TransformAsync` (`Cache/ObservableCacheEx.TransformAsync.cs`).
//!
//! Each item in the form that a future gives it, under the same key. The transforms of one change
//! set run at once, and the change set of their results is emitted when they have all ended; an
//! error of a future ends the sequence. A removal does not cancel the transforms in flight of its
//! key; the disposal of the subscription cancels the token of every one.
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's factory returns a `Task<TDestination>`; here
//! it returns a future of `Result<TDestination, ObservableError>`, which runs as a task of the
//! thread (docs/porting/reactive.md, section 5.8), and takes its arguments by value, so that the
//! future owns them. The overloads are named as those of `transform`: the factory is the suffix
//! `_with_key` (item and key), `_with_previous` (item, replaced item and key) or `_with_token`
//! (those and the `CancellationToken`); the observable of predicates is `_forced`; the
//! `TransformAsyncOptions` are `_with_options` (`_and_options` after another suffix).

macro_rules! transform_async_methods {
    () => {
        /// Each item in the form the future of `transform_factory` gives it: an addition or an
        /// update awaits the factory, a removal removes the item, a refresh is forwarded
        /// (`TransformAsync(Func<TSource, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_async<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T) -> Fut + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, _key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current)),
                None,
                None,
                None,
                false,
            )
        }

        /// [`transform_async`](Self::transform_async) with a factory of the item and its key
        /// (`TransformAsync(Func<TSource, TKey, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_async_with_key<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, K) -> Fut + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current, key)),
                None,
                None,
                None,
                false,
            )
        }

        /// [`transform_async`](Self::transform_async) with a factory of the item, the item it
        /// replaces (for an update) and the key
        /// (`TransformAsync(Func<TSource, Optional<TSource>, TKey, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_async_with_previous<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K) -> Fut + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| {
                    transform_factory(current, previous, key)
                }),
                None,
                None,
                None,
                false,
            )
        }

        /// [`transform_async_with_previous`](Self::transform_async_with_previous) whose factory
        /// also takes the token that the disposal of the subscription cancels
        /// (`TransformAsync(Func<TSource, Optional<TSource>, TKey, CancellationToken, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_async_with_token<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K, ferroui_system::threading::CancellationToken) -> Fut + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(self.change_sets(), crate::cache::internal::async_transform_factory(transform_factory), None, None, None, false)
        }

        /// [`transform_async`](Self::transform_async) that also transforms again, as updates, the
        /// items that each predicate of `force_transform` accepts
        /// (`TransformAsync(Func<TSource, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_async_forced<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T) -> Fut + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, _key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current)),
                None,
                Some(force_transform),
                None,
                false,
            )
        }

        /// [`transform_async_with_key`](Self::transform_async_with_key) with the predicates of
        /// [`transform_async_forced`](Self::transform_async_forced)
        /// (`TransformAsync(Func<TSource, TKey, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_async_with_key_forced<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, K) -> Fut + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current, key)),
                None,
                Some(force_transform),
                None,
                false,
            )
        }

        /// [`transform_async_with_previous`](Self::transform_async_with_previous) with the
        /// predicates of [`transform_async_forced`](Self::transform_async_forced)
        /// (`TransformAsync(Func<TSource, Optional<TSource>, TKey, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_async_with_previous_forced<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K) -> Fut + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| {
                    transform_factory(current, previous, key)
                }),
                None,
                Some(force_transform),
                None,
                false,
            )
        }

        /// [`transform_async_with_token`](Self::transform_async_with_token) with the predicates of
        /// [`transform_async_forced`](Self::transform_async_forced)
        /// (`TransformAsync(Func<TSource, Optional<TSource>, TKey, CancellationToken, Task<TDestination>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_async_with_token_forced<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K, ferroui_system::threading::CancellationToken) -> Fut + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(transform_factory),
                None,
                Some(force_transform),
                None,
                false,
            )
        }

        /// [`transform_async`](Self::transform_async) with the greatest number of transforms in
        /// flight and the handling of refreshes of `options`
        /// (`TransformAsync(Func<TSource, Task<TDestination>>, TransformAsyncOptions)`).
        fn transform_async_with_options<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T) -> Fut + 'static,
            options: crate::cache::TransformAsyncOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, _key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current)),
                None,
                None,
                options.maximum_concurrency,
                options.transform_on_refresh,
            )
        }

        /// [`transform_async_with_key`](Self::transform_async_with_key) with `options`
        /// (`TransformAsync(Func<TSource, TKey, Task<TDestination>>, TransformAsyncOptions)`).
        fn transform_async_with_key_and_options<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, K) -> Fut + 'static,
            options: crate::cache::TransformAsyncOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, _previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| transform_factory(current, key)),
                None,
                None,
                options.maximum_concurrency,
                options.transform_on_refresh,
            )
        }

        /// [`transform_async_with_previous`](Self::transform_async_with_previous) with `options`
        /// (`TransformAsync(Func<TSource, Optional<TSource>, TKey, Task<TDestination>>, TransformAsyncOptions)`).
        fn transform_async_with_previous_and_options<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K) -> Fut + 'static,
            options: crate::cache::TransformAsyncOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(move |current: T, previous: Option<T>, key: K, _token: ferroui_system::threading::CancellationToken| {
                    transform_factory(current, previous, key)
                }),
                None,
                None,
                options.maximum_concurrency,
                options.transform_on_refresh,
            )
        }

        /// [`transform_async_with_token`](Self::transform_async_with_token) with `options`
        /// (`TransformAsync(Func<TSource, Optional<TSource>, TKey, CancellationToken, Task<TDestination>>, TransformAsyncOptions)`).
        fn transform_async_with_token_and_options<U: Clone + 'static, Fut: std::future::Future<Output = Result<U, ferroui_system::ObservableError>> + 'static>(
            &self,
            transform_factory: impl Fn(T, Option<T>, K, ferroui_system::threading::CancellationToken) -> Fut + 'static,
            options: crate::cache::TransformAsyncOptions,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_async(
                self.change_sets(),
                crate::cache::internal::async_transform_factory(transform_factory),
                None,
                None,
                options.maximum_concurrency,
                options.transform_on_refresh,
            )
        }
    };
}
pub(crate) use transform_async_methods;
