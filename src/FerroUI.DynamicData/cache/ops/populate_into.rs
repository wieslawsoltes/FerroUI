//! `PopulateInto` (`Cache/ObservableCacheEx.PopulateInto.cs`).

macro_rules! populate_into_methods {
    () => {
        /// Applies each change set to `destination` (`PopulateInto(ISourceCache)`).
        fn populate_into<D: crate::cache::ISourceCache<T, K> + Clone + 'static>(&self, destination: &D) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            let destination = destination.clone();
            ferroui_reactive::ObservableExt::subscribe_next(&self.change_sets(), move |changes: crate::cache::ChangeSet<T, K>| {
                crate::cache::ISourceCache::edit(&destination, &mut |updater: &mut dyn crate::cache::ISourceUpdater<T, K>| updater.clone_changes(&changes));
            })
        }

        /// Applies each change set to `destination` (`PopulateInto(IIntermediateCache)`).
        fn populate_into_intermediate(&self, destination: &crate::cache::IntermediateCache<T, K>) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            let destination = destination.clone();
            ferroui_reactive::ObservableExt::subscribe_next(&self.change_sets(), move |changes: crate::cache::ChangeSet<T, K>| {
                destination.edit(|updater| updater.clone_changes(&changes));
            })
        }

        /// Applies each change set to `destination` (`PopulateInto(LockFreeObservableCache)`).
        fn populate_into_lock_free(&self, destination: &crate::cache::LockFreeObservableCache<T, K>) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            let destination = destination.clone();
            ferroui_reactive::ObservableExt::subscribe_next(&self.change_sets(), move |changes: crate::cache::ChangeSet<T, K>| {
                destination.edit(|updater| updater.clone_changes(&changes));
            })
        }
    };
}
pub(crate) use populate_into_methods;
