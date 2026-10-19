//! `NotEmpty` (`Cache/ObservableCacheEx.NotEmpty.cs`).

macro_rules! not_empty_methods {
    () => {
        /// The change sets that have changes (`NotEmpty`).
        fn not_empty(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            ferroui_reactive::ObservableExt::where_(&self.change_sets(), |changes: &crate::cache::ChangeSet<T, K>| changes.count() != 0)
        }
    };
}
pub(crate) use not_empty_methods;
