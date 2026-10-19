//! `OnItemRemoved` (`List/ObservableListEx.OnItemRemoved.cs`).

macro_rules! on_item_removed_methods {
    () => {
        /// Runs `remove_action` for each item removed, a replaced one included, before the change
        /// set is forwarded, and for every item left when the subscription ends
        /// (`OnItemRemoved(Action<T>, bool invokeOnUnsubscribe = true)`).
        fn on_item_removed(&self, remove_action: impl Fn(&T) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::on_item_removed(self.change_sets(), std::rc::Rc::new(remove_action), true)
        }

        /// [`on_item_removed`](Self::on_item_removed), for the items left when the subscription ends
        /// only when `invoke_on_unsubscribe` is true (`OnItemRemoved(Action<T>, bool)`).
        fn on_item_removed_with(
            &self,
            remove_action: impl Fn(&T) + 'static,
            invoke_on_unsubscribe: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::on_item_removed(self.change_sets(), std::rc::Rc::new(remove_action), invoke_on_unsubscribe)
        }
    };
}
pub(crate) use on_item_removed_methods;
