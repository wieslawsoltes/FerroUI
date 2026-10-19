//! `NotEmpty` (`List/ObservableListEx.NotEmpty.cs`).

macro_rules! not_empty_methods {
    () => {
        /// The change sets that have changes (`NotEmpty`).
        fn not_empty(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            ferroui_reactive::ObservableExt::where_(&self.change_sets(), |changes: &crate::list::ChangeSet<T>| changes.count() != 0)
        }
    };
}
pub(crate) use not_empty_methods;
