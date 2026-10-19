//! `TransformImmutable` (`Cache/ObservableCacheEx.TransformImmutable.cs`).

macro_rules! transform_immutable_methods {
    () => {
        /// Each change with its items transformed by `transform_factory` and every other part
        /// kept, the indexes included; nothing is cached, so the factory runs for every change
        /// (the previous item of an update too) and should be cheap and pure
        /// (`TransformImmutable(Func<TSource, TDestination>)`).
        fn transform_immutable<U: 'static>(
            &self,
            transform_factory: impl Fn(&T) -> U + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            let transform_factory: std::rc::Rc<dyn Fn(&T) -> Result<U, ferroui_system::ObservableError>> =
                std::rc::Rc::new(move |current: &T| -> Result<U, ferroui_system::ObservableError> { Ok(transform_factory(current)) });
            crate::cache::internal::transform_immutable(self.change_sets(), transform_factory)
        }

        /// [`transform_immutable`](Self::transform_immutable) with a factory that may fail: its
        /// error ends the sequence, and the change set that raised it is not emitted
        /// (`TransformImmutable` with a factory that throws).
        fn try_transform_immutable<U: 'static>(
            &self,
            transform_factory: impl Fn(&T) -> Result<U, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_immutable(self.change_sets(), std::rc::Rc::new(transform_factory))
        }
    };
}
pub(crate) use transform_immutable_methods;
