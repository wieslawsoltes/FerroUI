//! `OfType` (`Cache/ObservableCacheEx.OfType.cs`).

macro_rules! of_type_methods {
    () => {
        /// The items that are of `U`, an update that changes that becoming an addition or a
        /// removal; empty change sets are left out (`OfType<TDestination>(bool
        /// suppressEmptyChangeSets = true)`; named `of_type_cache` beside `ObservableExt::of_type`).
        /// The items are `Rc<dyn Any>`.
        fn of_type_cache<U: 'static>(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<std::rc::Rc<U>, K>>>
        where
            T: ferroui_reactive::linq::AnyObject,
        {
            crate::cache::internal::of_type::<T, K, U>(self.change_sets(), true)
        }

        /// [`of_type_cache`](Self::of_type_cache) that keeps the empty change sets when
        /// `suppress_empty_change_sets` is false (`OfType<TDestination>(bool)`).
        fn of_type_cache_with<U: 'static>(&self, suppress_empty_change_sets: bool) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<std::rc::Rc<U>, K>>>
        where
            T: ferroui_reactive::linq::AnyObject,
        {
            crate::cache::internal::of_type::<T, K, U>(self.change_sets(), suppress_empty_change_sets)
        }
    };
}
pub(crate) use of_type_methods;
