//! `ForEachChange` (`List/ObservableListEx.ForEachChange.cs`).

macro_rules! for_each_change_methods {
    () => {
        /// Runs `action` for each change, before the change set is forwarded
        /// (`ForEachChange(Action<Change<TObject>>)`).
        fn for_each_change(&self, action: impl Fn(&crate::list::Change<T>) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            ferroui_reactive::ObservableExt::do_(&self.change_sets(), move |changes: &crate::list::ChangeSet<T>| {
                for change in changes {
                    action(change);
                }
            })
        }
    };
}
pub(crate) use for_each_change_methods;
