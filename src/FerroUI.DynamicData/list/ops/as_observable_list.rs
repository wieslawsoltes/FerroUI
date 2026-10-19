//! `AsObservableList` (`List/ObservableListEx.AsObservableList.cs`).

macro_rules! as_observable_list_methods {
    () => {
        /// A read-only list of the items of the sequence (`AsObservableList(IObservable<IChangeSet<T>>)`).
        fn as_observable_list(&self) -> crate::list::ObservableList<T> {
            crate::list::ObservableList::from_source(self.change_sets())
        }
    };
}
pub(crate) use as_observable_list_methods;
