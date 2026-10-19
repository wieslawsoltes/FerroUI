//! `DisposeMany` (`Cache/ObservableCacheEx.DisposeMany.cs`).

macro_rules! dispose_many_methods {
    () => {
        /// Disposes each item when it is removed or replaced (by another item), and every item left
        /// when the subscription ends; the change sets are forwarded first (`DisposeMany`).
        ///
        /// The items are disposables through [`AsDisposable`](crate::AsDisposable); an item for
        /// which it gives `None` is left alone, as upstream leaves an item that is not an
        /// `IDisposable`.
        fn dispose_many(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: crate::AsDisposable,
        {
            crate::cache::internal::dispose_many(self.change_sets(), std::rc::Rc::new(|item: &T| crate::AsDisposable::to_disposable(item)))
        }

        /// [`dispose_many`](Self::dispose_many) for items that `to_disposable` makes disposables of
        /// (`item as IDisposable`).
        fn dispose_many_with(
            &self,
            to_disposable: impl Fn(&T) -> Option<std::rc::Rc<dyn ferroui_system::IDisposable>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::dispose_many(self.change_sets(), std::rc::Rc::new(to_disposable))
        }
    };
}
pub(crate) use dispose_many_methods;
