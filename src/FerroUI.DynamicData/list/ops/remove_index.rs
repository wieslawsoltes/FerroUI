//! `RemoveIndex` (`List/ObservableListEx.RemoveIndex.cs`).

macro_rules! remove_index_methods {
    () => {
        /// The change sets without the indexes of their changes; moves are left out (`RemoveIndex`).
        fn remove_index(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            ferroui_reactive::ObservableExt::select(&self.change_sets(), |changes: crate::list::ChangeSet<T>| {
                crate::list::ChangeSet::from(crate::list::yield_without_index(&changes))
            })
        }
    };
}
pub(crate) use remove_index_methods;
