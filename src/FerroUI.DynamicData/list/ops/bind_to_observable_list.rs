//! `BindToObservableList` of lists (`Binding/IObservableListEx.cs`).

macro_rules! bind_to_observable_list_methods {
    () => {
        /// Fills a new observable list with the change sets as they pass; the list is disposed
        /// when the sequence ends (`BindToObservableList(out IObservableList<TObject>)`).
        fn bind_to_observable_list(
            &self,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>, std::rc::Rc<dyn crate::list::IObservableList<T>>) {
            crate::binding::i_observable_list_ex::bind_list_to_observable_list(self.change_sets())
        }
    };
}
pub(crate) use bind_to_observable_list_methods;
