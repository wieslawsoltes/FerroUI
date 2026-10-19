//! `TransformSafe` (`Cache/ObservableCacheEx.TransformSafe.cs`).
//!
//! The factory may fail; an error goes to `error_handler` with the item and its key, the item is
//! left out, and the sequence goes on. The forms are named as those of `transform`: `_with_key`
//! and `_with_previous` for the factory, `_forced` and `_forced_all` for the forced transform. The
//! factory of a forced transform is not guarded, as upstream's (its error ends the sequence).

macro_rules! transform_safe_methods {
    () => {
        /// [`transform`](Self::transform) whose factory may fail: an error goes to
        /// `error_handler` and leaves the item out
        /// (`TransformSafe(Func<TSource, TDestination>, Action<Error<TSource, TKey>>, IObservable<Func<TSource, bool>>? forceTransform = null)`).
        fn transform_safe<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(
                self.change_sets(),
                move |current: &T, _previous: Option<&T>, _key: &K| transform_factory(current),
                Some(std::rc::Rc::new(error_handler)),
                false,
            )
        }

        /// [`transform_safe`](Self::transform_safe) with a factory of the item and its key
        /// (`TransformSafe(Func<TSource, TKey, TDestination>, Action<Error<TSource, TKey>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_safe_with_key<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(
                self.change_sets(),
                move |current: &T, _previous: Option<&T>, key: &K| transform_factory(current, key),
                Some(std::rc::Rc::new(error_handler)),
                false,
            )
        }

        /// [`transform_safe`](Self::transform_safe) with a factory of the item, the item it
        /// replaces and the key
        /// (`TransformSafe(Func<TSource, Optional<TSource>, TKey, TDestination>, Action<Error<TSource, TKey>>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_safe_with_previous<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), transform_factory, Some(std::rc::Rc::new(error_handler)), false)
        }

        /// [`transform_safe`](Self::transform_safe) that also transforms again, as updates, the
        /// items that each predicate of `force_transform` accepts
        /// (`TransformSafe(Func<TSource, TDestination>, Action<Error<TSource, TKey>>, IObservable<Func<TSource, bool>>)`).
        fn transform_safe_forced<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, _previous: Option<&T>, _key: &K| transform_factory(current)),
                crate::cache::internal::for_forced(force_transform),
                Some(std::rc::Rc::new(error_handler)),
            )
        }

        /// [`transform_safe_with_key`](Self::transform_safe_with_key) that also transforms
        /// again the items that each predicate of `force_transform` accepts
        /// (`TransformSafe(Func<TSource, TKey, TDestination>, Action<Error<TSource, TKey>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_safe_with_key_forced<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, _previous: Option<&T>, key: &K| transform_factory(current, key)),
                force_transform,
                Some(std::rc::Rc::new(error_handler)),
            )
        }

        /// [`transform_safe_with_previous`](Self::transform_safe_with_previous) that also
        /// transforms again the items that each predicate of `force_transform` accepts
        /// (`TransformSafe(Func<TSource, Optional<TSource>, TKey, TDestination>, Action<Error<TSource, TKey>>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_safe_with_previous_forced<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(transform_factory),
                force_transform,
                Some(std::rc::Rc::new(error_handler)),
            )
        }

        /// [`transform_safe`](Self::transform_safe) that transforms every item again whenever
        /// `force_transform` produces a value
        /// (`TransformSafe(Func<TSource, TDestination>, Action<Error<TSource, TKey>>, IObservable<Unit>)`).
        fn transform_safe_forced_all<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, _previous: Option<&T>, _key: &K| transform_factory(current)),
                crate::cache::internal::for_forced_all(force_transform),
                Some(std::rc::Rc::new(error_handler)),
            )
        }

        /// [`transform_safe_with_key`](Self::transform_safe_with_key) that transforms every item
        /// again whenever `force_transform` produces a value
        /// (`TransformSafe(Func<TSource, TKey, TDestination>, Action<Error<TSource, TKey>>, IObservable<Unit>)`).
        fn transform_safe_with_key_forced_all<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, _previous: Option<&T>, key: &K| transform_factory(current, key)),
                crate::cache::internal::for_forced_all(force_transform),
                Some(std::rc::Rc::new(error_handler)),
            )
        }

        /// [`transform_safe_with_previous`](Self::transform_safe_with_previous) that transforms
        /// every item again whenever `force_transform` produces a value
        /// (`TransformSafe(Func<TSource, Optional<TSource>, TKey, TDestination>, Action<Error<TSource, TKey>>, IObservable<Unit>)`).
        fn transform_safe_with_previous_forced_all<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(transform_factory),
                crate::cache::internal::for_forced_all(force_transform),
                Some(std::rc::Rc::new(error_handler)),
            )
        }
    };
}
pub(crate) use transform_safe_methods;
