//! `SubscribeMany` (`List/ObservableListEx.SubscribeMany.cs`).

macro_rules! subscribe_many_methods {
    () => {
        /// Makes a subscription for each item with `subscription_factory`, when it is added; the
        /// subscription is disposed when the item is removed or replaced, or when the subscription
        /// to the result ends. The change sets are forwarded as they are
        /// (`SubscribeMany(Func<T, IDisposable>)`).
        fn subscribe_many(
            &self,
            subscription_factory: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IDisposable> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::subscribe_many(self.change_sets(), std::rc::Rc::new(subscription_factory))
        }
    };
}
pub(crate) use subscribe_many_methods;
