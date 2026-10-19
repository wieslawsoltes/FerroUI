//! `ForEachItemChange` (`List/ObservableListEx.ForEachItemChange.cs`).

macro_rules! for_each_item_change_methods {
    () => {
        /// Runs `action` for the change of each item, the ranges spread into the changes of their
        /// items, before the change set is forwarded (`ForEachItemChange(Action<ItemChange<TObject>>)`).
        fn for_each_item_change(
            &self,
            action: impl Fn(&crate::list::ItemChange<T>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            ferroui_reactive::ObservableExt::do_(&self.change_sets(), move |changes: &crate::list::ChangeSet<T>| {
                for change in crate::list::internal::flatten_item_changes(changes) {
                    action(&change);
                }
            })
        }
    };
}
pub(crate) use for_each_item_change_methods;
