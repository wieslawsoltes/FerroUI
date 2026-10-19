//! `SkipInitial` (`List/ObservableListEx.SkipInitial.cs`).

macro_rules! skip_initial_methods {
    () => {
        /// The change sets after the first one (`SkipInitial`).
        fn skip_initial(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            ferroui_reactive::ObservableExt::skip(&crate::list::internal::defer_until_loaded(self.change_sets()), 1)
        }
    };
}
pub(crate) use skip_initial_methods;
