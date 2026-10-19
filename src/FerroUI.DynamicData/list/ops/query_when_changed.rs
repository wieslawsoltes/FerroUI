//! `QueryWhenChanged` (`List/ObservableListEx.QueryWhenChanged.cs`).

macro_rules! query_when_changed_methods {
    () => {
        /// A copy of the items after each change set (`QueryWhenChanged()`, whose
        /// `IReadOnlyCollection<T>` is a `Vec<T>`).
        fn query_when_changed(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::list::internal::query_when_changed(self.change_sets())
        }

        /// What `result_selector` makes of the items after each change set
        /// (`QueryWhenChanged(Func<IReadOnlyCollection<TObject>, TDestination>)`).
        fn query_when_changed_with<R: 'static>(&self, result_selector: impl Fn(Vec<T>) -> R + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            ferroui_reactive::ObservableExt::select(&self.query_when_changed(), result_selector)
        }
    };
}
pub(crate) use query_when_changed_methods;
