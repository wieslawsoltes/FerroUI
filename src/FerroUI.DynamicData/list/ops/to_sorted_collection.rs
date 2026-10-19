//! `ToSortedCollection` of lists (`List/ObservableListEx.ToSortedCollection.cs`).

macro_rules! to_sorted_collection_methods {
    () => {
        /// The items of the list after each change set, sorted by the key that `sort` gives
        /// (`ToSortedCollection(Func<TObject, TSortKey>, SortDirection sortOrder = Ascending)`); a
        /// stable sort, as `OrderBy` and `OrderByDescending` are.
        fn to_sorted_collection<S: Ord + 'static>(
            &self,
            sort: impl Fn(&T) -> S + 'static,
            sort_order: crate::binding::SortDirection,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::list::internal::query_items(
                self.change_sets(),
                std::rc::Rc::new(move |mut items: Vec<T>| {
                    match sort_order {
                        crate::binding::SortDirection::Ascending => items.sort_by(|a, b| sort(a).cmp(&sort(b))),
                        crate::binding::SortDirection::Descending => items.sort_by(|a, b| sort(b).cmp(&sort(a))),
                    }
                    items
                }),
            )
        }

        /// The items of the list after each change set, sorted by `comparer`
        /// (`ToSortedCollection(IComparer<TObject>)`).
        fn to_sorted_collection_with_comparer(
            &self,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::list::internal::query_items(
                self.change_sets(),
                std::rc::Rc::new(move |mut items: Vec<T>| {
                    items.sort_by(|a, b| comparer(a, b));
                    items
                }),
            )
        }
    };
}
pub(crate) use to_sorted_collection_methods;
