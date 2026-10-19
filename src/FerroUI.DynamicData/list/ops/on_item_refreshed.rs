//! `OnItemRefreshed` (`List/ObservableListEx.OnItemRefreshed.cs`).

macro_rules! on_item_refreshed_methods {
    () => {
        /// Runs `refresh_action` for each item refreshed, before the change set is forwarded
        /// (`OnItemRefreshed(Action<T>)`).
        fn on_item_refreshed(&self, refresh_action: impl Fn(&T) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::on_item_refreshed(self.change_sets(), std::rc::Rc::new(refresh_action))
        }
    };
}
pub(crate) use on_item_refreshed_methods;
