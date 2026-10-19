//! `Top` of lists (`List/ObservableListEx.Top.cs`).

macro_rules! top_methods {
    () => {
        /// The first `number_of_items` items (`Top(int)`): the window from 0 of that size.
        ///
        /// # Panics
        /// When `number_of_items` is 0 (`ArgumentOutOfRangeException`).
        fn top(&self, number_of_items: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            assert!(number_of_items > 0, "numberOfItems must be greater than zero");
            let virtualised = self.virtualise(ferroui_reactive::Observable::return_(crate::cache::VirtualRequest::new(0, number_of_items)));
            crate::list::ObservableListEx::change_sets(&virtualised)
        }
    };
}
pub(crate) use top_methods;
