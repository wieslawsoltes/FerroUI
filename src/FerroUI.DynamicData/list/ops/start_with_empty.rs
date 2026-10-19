//! `StartWithEmpty` (`List/ObservableListEx.StartWithEmpty.cs`).

macro_rules! start_with_empty_methods {
    () => {
        /// An empty change set, then the change sets (`StartWithEmpty`).
        fn start_with_empty(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            ferroui_reactive::ObservableExt::start_with(&self.change_sets(), [crate::list::ChangeSet::empty()])
        }
    };
}
pub(crate) use start_with_empty_methods;
