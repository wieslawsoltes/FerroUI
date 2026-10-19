//! `Page` of lists (`List/ObservableListEx.Page.cs`).

macro_rules! page_methods {
    () => {
        /// The page of the items that the latest of `requests` asks for, with a description of
        /// the page (`Page(IObservable<IPageRequest>)`). Pages count from 1.
        fn page(
            &self,
            requests: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::PageRequest>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::PageChangeSet<T>>> {
            crate::list::internal::pager(self.change_sets(), requests)
        }
    };
}
pub(crate) use page_methods;
