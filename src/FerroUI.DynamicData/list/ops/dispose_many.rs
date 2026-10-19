//! `DisposeMany` (`List/ObservableListEx.DisposeMany.cs`).

macro_rules! dispose_many_methods {
    () => {
        /// Disposes each item when it is removed or replaced, and every item left when the sequence
        /// ends or the subscription ends; the change sets are forwarded first (`DisposeMany`).
        ///
        /// The items are disposables through [`AsDisposable`](crate::AsDisposable); an item for
        /// which it gives `None` is left alone.
        fn dispose_many(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>
        where
            T: crate::AsDisposable,
        {
            crate::list::internal::dispose_many(
                self.change_sets(),
                std::rc::Rc::new(|item: &T| {
                    if let Some(disposable) = crate::AsDisposable::to_disposable(item) {
                        disposable.dispose();
                    }
                }),
            )
        }

        /// [`dispose_many`](Self::dispose_many) for items that `to_disposable` makes disposables of
        /// (`item as IDisposable`).
        fn dispose_many_with(
            &self,
            to_disposable: impl Fn(&T) -> Option<std::rc::Rc<dyn ferroui_system::IDisposable>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::dispose_many(
                self.change_sets(),
                std::rc::Rc::new(move |item: &T| {
                    if let Some(disposable) = to_disposable(item) {
                        disposable.dispose();
                    }
                }),
            )
        }
    };
}
pub(crate) use dispose_many_methods;
