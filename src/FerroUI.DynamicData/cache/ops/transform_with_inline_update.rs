//! `TransformWithInlineUpdate` (`Cache/ObservableCacheEx.TransformWithInlineUpdate.cs`).
//!
//! The transformed items are shared objects (upstream's `TDestination : class`, an `Rc<..>` with
//! interior mutability here): an update of an item of the source changes its transformed item in
//! place and emits a refresh of it. `_with` takes the flag to update on refresh too, `_safe` an
//! error handler, `try_` a factory and an update that may fail and end the sequence.

macro_rules! transform_with_inline_update_methods {
    () => {
        /// Each item transformed once by `transform_factory`; an update calls `update_action`
        /// with the transformed item and the new item instead, and emits a refresh
        /// (`TransformWithInlineUpdate(Func<TSource, TDestination>, Action<TDestination, TSource>)`).
        fn transform_with_inline_update<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> U + 'static,
            update_action: impl Fn(&U, &T) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            self.transform_with_inline_update_with(transform_factory, update_action, false)
        }

        /// [`transform_with_inline_update`](Self::transform_with_inline_update) that, when
        /// `transform_on_refresh` is true, updates the transformed item on a refresh too
        /// (`TransformWithInlineUpdate(Func<TSource, TDestination>, Action<TDestination, TSource>, bool transformOnRefresh)`).
        fn transform_with_inline_update_with<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> U + 'static,
            update_action: impl Fn(&U, &T) + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            let transform_factory: std::rc::Rc<dyn Fn(&T) -> Result<U, ferroui_system::ObservableError>> = std::rc::Rc::new(move |current: &T| -> Result<U, ferroui_system::ObservableError> { Ok(transform_factory(current)) });
            let update_action: std::rc::Rc<dyn Fn(&U, &T) -> Result<(), ferroui_system::ObservableError>> = std::rc::Rc::new(move |previous: &U, current: &T| -> Result<(), ferroui_system::ObservableError> {
                update_action(previous, current);
                Ok(())
            });
            crate::cache::internal::transform_with_inline_update(self.change_sets(), transform_factory, update_action, None, transform_on_refresh)
        }

        /// [`transform_with_inline_update`](Self::transform_with_inline_update) whose factory and
        /// update may fail: an error goes to `error_handler`
        /// (`TransformWithInlineUpdate(Func<TSource, TDestination>, Action<TDestination, TSource>, Action<Error<TSource, TKey>>)`).
        fn transform_with_inline_update_safe<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
            update_action: impl Fn(&U, &T) -> Result<(), ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            self.transform_with_inline_update_safe_with(transform_factory, update_action, error_handler, false)
        }

        /// [`transform_with_inline_update_safe`](Self::transform_with_inline_update_safe) with the
        /// flag of [`transform_with_inline_update_with`](Self::transform_with_inline_update_with)
        /// (`TransformWithInlineUpdate(Func<TSource, TDestination>, Action<TDestination, TSource>, Action<Error<TSource, TKey>>, bool transformOnRefresh)`).
        fn transform_with_inline_update_safe_with<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
            update_action: impl Fn(&U, &T) -> Result<(), ferroui_system::ObservableError> + 'static,
            error_handler: impl Fn(crate::kernel::Error<T, K>) + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_inline_update(
                self.change_sets(),
                std::rc::Rc::new(transform_factory),
                std::rc::Rc::new(update_action),
                Some(std::rc::Rc::new(error_handler)),
                transform_on_refresh,
            )
        }

        /// [`transform_with_inline_update`](Self::transform_with_inline_update) whose factory and
        /// update may fail: an error ends the sequence.
        fn try_transform_with_inline_update<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
            update_action: impl Fn(&U, &T) -> Result<(), ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            self.try_transform_with_inline_update_with(transform_factory, update_action, false)
        }

        /// [`try_transform_with_inline_update`](Self::try_transform_with_inline_update) with the
        /// flag of [`transform_with_inline_update_with`](Self::transform_with_inline_update_with).
        fn try_transform_with_inline_update_with<U: Clone + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
            update_action: impl Fn(&U, &T) -> Result<(), ferroui_system::ObservableError> + 'static,
            transform_on_refresh: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_with_inline_update(
                self.change_sets(),
                std::rc::Rc::new(transform_factory),
                std::rc::Rc::new(update_action),
                None,
                transform_on_refresh,
            )
        }
    };
}
pub(crate) use transform_with_inline_update_methods;
