//! `Flatten` (`Cache/ObservableCacheEx.Flatten.cs`).

macro_rules! flatten_methods {
    () => {
        /// The changes of the change sets, one at a time (`Flatten`).
        fn flatten(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::Change<T, K>>> {
            ferroui_reactive::ObservableExt::select_many_iter(&self.change_sets(), |changes: crate::cache::ChangeSet<T, K>| changes.to_vec())
        }
    };
}
pub(crate) use flatten_methods;
