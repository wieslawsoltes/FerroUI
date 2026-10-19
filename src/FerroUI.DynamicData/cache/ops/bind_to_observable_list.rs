//! `BindToObservableList` of caches (`Binding/IObservableListEx.cs`).

macro_rules! bind_to_observable_list_methods {
    () => {
        /// Fills a new observable list with the items, without their keys, as the change sets
        /// pass; the list is disposed when the sequence ends (`BindToObservableList(out
        /// IObservableList<TObject>)`, and its form for a sorted source, which keeps the order of
        /// the sort).
        fn bind_to_observable_list(
            &self,
        ) -> (std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>, std::rc::Rc<dyn crate::list::IObservableList<T>>)
        where
            T: PartialEq,
            Self: Sized + 'static,
        {
            match crate::cache::internal::bind::as_sorted_stream::<Self, T, K>(self) {
                Some(sorted) => crate::binding::i_observable_list_ex::bind_sorted_to_observable_list(sorted),
                None => crate::binding::i_observable_list_ex::bind_cache_to_observable_list(self.change_sets()),
            }
        }
    };
}
pub(crate) use bind_to_observable_list_methods;
