//! `AsyncDisposeMany` (`Cache/ObservableCacheEx.AsyncDisposeMany.cs`).
//!
//! Upstream's operator exists only where the platform has `IAsyncDisposable`
//! (`SUPPORTS_ASYNC_DISPOSABLE`); here it always exists, with [`IAsyncDisposable`](crate::IAsyncDisposable)
//! from this crate.

macro_rules! async_dispose_many_methods {
    () => {
        /// Disposes the items that are disposable, at once or by a task, when they are removed or
        /// replaced by an item that is not equal to them, and every item left when the sequence
        /// ends, fails or the subscription is disposed; the change sets are forwarded first.
        /// `disposals_completed_accessor` receives, once per subscription, a sequence of one value
        /// that ends when the disposals by tasks have all ended, and fails with the error of one
        /// (`AsyncDisposeMany(Action<IObservable<Unit>>)`).
        ///
        /// The items tell how they are disposed through
        /// [`AsAsyncDisposable`](crate::AsAsyncDisposable); an item for which it gives `None` is
        /// left alone.
        fn async_dispose_many(
            &self,
            disposals_completed_accessor: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<()>>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: crate::AsAsyncDisposable + PartialEq,
        {
            crate::cache::internal::async_dispose_many(
                self.change_sets(),
                std::rc::Rc::new(|item: &T| crate::AsAsyncDisposable::to_item_disposal(item)),
                std::rc::Rc::new(disposals_completed_accessor),
            )
        }

        /// [`async_dispose_many`](Self::async_dispose_many) for items that `to_item_disposal`
        /// tells how to dispose (`item is IDisposable`, `item is IAsyncDisposable`).
        fn async_dispose_many_with(
            &self,
            to_item_disposal: impl Fn(&T) -> Option<crate::ItemDisposal> + 'static,
            disposals_completed_accessor: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<()>>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            crate::cache::internal::async_dispose_many(self.change_sets(), std::rc::Rc::new(to_item_disposal), std::rc::Rc::new(disposals_completed_accessor))
        }
    };
}
pub(crate) use async_dispose_many_methods;
