//! `Transform` (`Cache/ObservableCacheEx.Transform.cs`).
//!
//! Upstream's overloads differ by the factory (of the item, of the item and its key, or of the
//! item, the item it replaced and the key) and by the forced transform (none, a flag to transform
//! on refresh, an observable of predicates, an observable of `Unit`). Each pair is a method: the
//! factory is the suffix `_with_key` or `_with_previous`, the flag `_and_refresh` (`_with_refresh`
//! after the plain factory), the predicates `_forced`, `Unit` `_forced_all`. The `try_` forms take a
//! factory that may fail, whose error ends the sequence (upstream's exception, which reaches
//! `OnError`); the factory of the other forms cannot fail.

macro_rules! transform_methods {
    () => {
        /// Each item in the form `transform_factory` gives it, under the same key: an addition or
        /// an update transforms the item, a removal removes it, a refresh is forwarded and a move
        /// ignored (`Transform(Func<TSource, TDestination>, IObservable<Func<TSource, bool>>? forceTransform = null)`).
        fn transform<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> U + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), move |current: &T, _previous: Option<&T>, _key: &K| Ok(transform_factory(current)), None, false)
        }

        /// [`transform`](Self::transform) with a factory of the item and its key
        /// (`Transform(Func<TSource, TKey, TDestination>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_with_key<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, &K) -> U + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), move |current: &T, _previous: Option<&T>, key: &K| Ok(transform_factory(current, key)), None, false)
        }

        /// [`transform`](Self::transform) with a factory of the item, the item it replaces (for an
        /// update) and the key
        /// (`Transform(Func<TSource, Optional<TSource>, TKey, TDestination>, IObservable<Func<TSource, TKey, bool>>? forceTransform = null)`).
        fn transform_with_previous<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> U + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), move |current: &T, previous: Option<&T>, key: &K| Ok(transform_factory(current, previous, key)), None, false)
        }

        /// [`transform`](Self::transform) that, when `transform_on_refresh` is true, transforms
        /// a refreshed item again and emits it as an update
        /// (`Transform(Func<TSource, TDestination>, bool transformOnRefresh)`).
        fn transform_with_refresh<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> U + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), move |current: &T, _previous: Option<&T>, _key: &K| Ok(transform_factory(current)), None, transform_on_refresh)
        }

        /// [`transform_with_key`](Self::transform_with_key) with the flag of
        /// [`transform_with_refresh`](Self::transform_with_refresh)
        /// (`Transform(Func<TSource, TKey, TDestination>, bool transformOnRefresh)`).
        fn transform_with_key_and_refresh<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, &K) -> U + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), move |current: &T, _previous: Option<&T>, key: &K| Ok(transform_factory(current, key)), None, transform_on_refresh)
        }

        /// [`transform_with_previous`](Self::transform_with_previous) with the flag of
        /// [`transform_with_refresh`](Self::transform_with_refresh)
        /// (`Transform(Func<TSource, Optional<TSource>, TKey, TDestination>, bool transformOnRefresh)`).
        fn transform_with_previous_and_refresh<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> U + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(
                self.change_sets(),
                move |current: &T, previous: Option<&T>, key: &K| Ok(transform_factory(current, previous, key)),
                None,
                transform_on_refresh,
            )
        }

        /// [`transform`](Self::transform) that also transforms again, as updates, the items that
        /// each predicate of `force_transform` accepts
        /// (`Transform(Func<TSource, TDestination>, IObservable<Func<TSource, bool>>)`).
        fn transform_forced<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> U + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, _previous: Option<&T>, _key: &K| Ok(transform_factory(current))),
                crate::cache::internal::for_forced(force_transform),
                None,
            )
        }

        /// [`transform_with_key`](Self::transform_with_key) that also transforms again, as
        /// updates, the items that each predicate of `force_transform` accepts
        /// (`Transform(Func<TSource, TKey, TDestination>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_with_key_forced<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, &K) -> U + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, _previous: Option<&T>, key: &K| Ok(transform_factory(current, key))),
                force_transform,
                None,
            )
        }

        /// [`transform_with_previous`](Self::transform_with_previous) that also transforms again,
        /// as updates, the items that each predicate of `force_transform` accepts
        /// (`Transform(Func<TSource, Optional<TSource>, TKey, TDestination>, IObservable<Func<TSource, TKey, bool>>)`).
        fn transform_with_previous_forced<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> U + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, previous: Option<&T>, key: &K| Ok(transform_factory(current, previous, key))),
                force_transform,
                None,
            )
        }

        /// [`transform`](Self::transform) that transforms every item again, as updates, whenever
        /// `force_transform` produces a value
        /// (`Transform(Func<TSource, TDestination>, IObservable<Unit>)`).
        fn transform_forced_all<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> U + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, _previous: Option<&T>, _key: &K| Ok(transform_factory(current))),
                crate::cache::internal::for_forced_all(force_transform),
                None,
            )
        }

        /// [`transform_with_key`](Self::transform_with_key) that transforms every item again
        /// whenever `force_transform` produces a value
        /// (`Transform(Func<TSource, TKey, TDestination>, IObservable<Unit>)`).
        fn transform_with_key_forced_all<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, &K) -> U + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, _previous: Option<&T>, key: &K| Ok(transform_factory(current, key))),
                crate::cache::internal::for_forced_all(force_transform),
                None,
            )
        }

        /// [`transform_with_previous`](Self::transform_with_previous) that transforms every item
        /// again whenever `force_transform` produces a value
        /// (`Transform(Func<TSource, Optional<TSource>, TKey, TDestination>, IObservable<Unit>)`).
        fn transform_with_previous_forced_all<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> U + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(move |current: &T, previous: Option<&T>, key: &K| Ok(transform_factory(current, previous, key))),
                crate::cache::internal::for_forced_all(force_transform),
                None,
            )
        }

        /// [`transform`](Self::transform) with a factory that may fail: its error ends the
        /// sequence (`Transform(Func<TSource, TDestination>)` with a factory that throws).
        fn try_transform<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), move |current: &T, _previous: Option<&T>, _key: &K| transform_factory(current), None, false)
        }

        /// [`transform_with_key`](Self::transform_with_key) with a factory that may fail.
        fn try_transform_with_key<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), move |current: &T, _previous: Option<&T>, key: &K| transform_factory(current, key), None, false)
        }

        /// [`transform_with_previous`](Self::transform_with_previous) with a factory that may fail.
        fn try_transform_with_previous<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), transform_factory, None, false)
        }

        /// [`transform_with_previous_and_refresh`](Self::transform_with_previous_and_refresh) with
        /// a factory that may fail.
        fn try_transform_with_previous_and_refresh<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform(self.change_sets(), transform_factory, None, transform_on_refresh)
        }

        /// [`transform_with_previous_forced`](Self::transform_with_previous_forced) with a factory
        /// that may fail.
        fn try_transform_with_previous_forced<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&T, &K) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(self.change_sets(), crate::cache::internal::transform_factory(transform_factory), force_transform, None)
        }

        /// [`transform_with_previous_forced_all`](Self::transform_with_previous_forced_all) with a
        /// factory that may fail.
        fn try_transform_with_previous_forced_all<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T, Option<&T>, &K) -> Result<U, ferroui_system::ObservableError> + 'static,
            force_transform: std::rc::Rc<dyn ferroui_system::IObservable<()>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_forced_transform(
                self.change_sets(),
                crate::cache::internal::transform_factory(transform_factory),
                crate::cache::internal::for_forced_all(force_transform),
                None,
            )
        }
    };
}
pub(crate) use transform_methods;
