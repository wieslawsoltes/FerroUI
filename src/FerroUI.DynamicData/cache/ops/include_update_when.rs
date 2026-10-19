//! `IncludeUpdateWhen` (`Cache/ObservableCacheEx.IncludeUpdateWhen.cs`).

macro_rules! include_update_when_methods {
    () => {
        /// The changes without the updates that `include_function` rejects; it is given the current
        /// and the previous item. The change sets left empty are dropped
        /// (`IncludeUpdateWhen(Func<TObject, TObject, bool>)`).
        fn include_update_when(
            &self,
            include_function: impl Fn(&T, &T) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            let selected = ferroui_reactive::ObservableExt::select(&self.change_sets(), move |changes: crate::cache::ChangeSet<T, K>| {
                changes
                    .iter()
                    .filter(|change| {
                        change.reason() != crate::cache::ChangeReason::Update
                            || include_function(change.current(), change.previous().expect("an update has a previous item"))
                    })
                    .cloned()
                    .collect::<crate::cache::ChangeSet<T, K>>()
            });
            crate::cache::ObservableCacheEx::<T, K>::not_empty(&selected)
        }
    };
}
pub(crate) use include_update_when_methods;
