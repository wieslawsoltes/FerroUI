//! `DeferUntilLoaded` (`List/ObservableListEx.DeferUntilLoaded.cs`).

macro_rules! defer_until_loaded_methods {
    () => {
        /// The change sets from the first one; empty change sets are left out
        /// (`DeferUntilLoaded(IObservable<IChangeSet<T>>)`).
        fn defer_until_loaded(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::defer_until_loaded(self.change_sets())
        }
    };
}
pub(crate) use defer_until_loaded_methods;

/// The form on lists, a provided method of [`SourceListEx`](crate::list::SourceListEx).
macro_rules! defer_until_loaded_source_methods {
    () => {
        /// The items and their changes from the first change set (`DeferUntilLoaded(IObservableList)`).
        fn defer_until_loaded(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::internal::defer_until_loaded(crate::list::IObservableList::connect(self))
        }
    };
}
pub(crate) use defer_until_loaded_source_methods;
