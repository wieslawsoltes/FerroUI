//! `ToCollection` (`Cache/ObservableCacheEx.ToCollection.cs`).

macro_rules! to_collection_methods {
    () => {
        /// The items of the cache after each change set (`ToCollection`): a copy of the items, in
        /// the order of the cache.
        fn to_collection(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::cache::internal::to_collection::query_items(self.change_sets(), std::rc::Rc::new(|items: Vec<T>| items))
        }
    };
}
pub(crate) use to_collection_methods;
