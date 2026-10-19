//! `ToCollection` of lists (`List/ObservableListEx.ToCollection.cs`).

macro_rules! to_collection_methods {
    () => {
        /// The items of the list after each change set (`ToCollection`): a copy of the items.
        fn to_collection(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::list::internal::query_items(self.change_sets(), std::rc::Rc::new(|items: Vec<T>| items))
        }
    };
}
pub(crate) use to_collection_methods;
