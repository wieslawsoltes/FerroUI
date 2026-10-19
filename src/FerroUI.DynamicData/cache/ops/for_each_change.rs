//! `ForEachChange` (`Cache/ObservableCacheEx.ForEachChange.cs`).

macro_rules! for_each_change_methods {
    () => {
        /// Runs `action` for each change, before the change set is forwarded
        /// (`ForEachChange(Action<Change<TObject, TKey>>)`).
        fn for_each_change(
            &self,
            action: impl Fn(&crate::cache::Change<T, K>) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            ferroui_reactive::ObservableExt::do_(&self.change_sets(), move |changes: &crate::cache::ChangeSet<T, K>| {
                for change in changes {
                    action(change);
                }
            })
        }
    };
}
pub(crate) use for_each_change_methods;
