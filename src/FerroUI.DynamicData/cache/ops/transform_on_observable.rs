//! `TransformOnObservable` (`Cache/ObservableCacheEx.TransformOnObservable.cs`).

macro_rules! transform_on_observable_methods {
    () => {
        /// For each item, the latest distinct value of the observable `transform_factory` makes of
        /// it, under the key of the item: an addition or an update subscribes to a new observable,
        /// a removal ends it and removes the value. The values that one change set of the source
        /// makes at once come as one change set; the sequence ends when the source and every
        /// observable have ended, and fails with the first error of any of them
        /// (`TransformOnObservable(Func<TSource, IObservable<TDestination>>)`).
        fn transform_on_observable<U: Clone + PartialEq + 'static>(
            &self,
            transform_factory: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            let transform: std::rc::Rc<dyn Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>> =
                std::rc::Rc::new(move |item: &T, _key: &K| transform_factory(item));
            crate::cache::internal::transform_on_observable(self.change_sets(), transform, false)
        }

        /// [`transform_on_observable`](Self::transform_on_observable) with a factory of the item
        /// and its key (`TransformOnObservable(Func<TSource, TKey, IObservable<TDestination>>)`).
        fn transform_on_observable_with_key<U: Clone + PartialEq + 'static>(
            &self,
            transform_factory: impl Fn(&T, &K) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<U, K>>> {
            crate::cache::internal::transform_on_observable(self.change_sets(), std::rc::Rc::new(transform_factory), false)
        }
    };
}
pub(crate) use transform_on_observable_methods;
