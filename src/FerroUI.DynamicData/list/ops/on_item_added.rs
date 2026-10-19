//! `OnItemAdded` (`List/ObservableListEx.OnItemAdded.cs`).

macro_rules! on_item_added_methods {
    () => {
        /// Runs `add_action` for each item added, a replacement included, before the change set is
        /// forwarded (`OnItemAdded(Action<T>)`).
        fn on_item_added(&self, add_action: impl Fn(&T) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::on_item_added(self.change_sets(), std::rc::Rc::new(add_action))
        }
    };
}
pub(crate) use on_item_added_methods;
