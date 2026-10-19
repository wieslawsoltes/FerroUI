//! `IgnoreUpdateWhen` (`Cache/ObservableCacheEx.IgnoreUpdateWhen.cs`).

macro_rules! ignore_update_when_methods {
    () => {
        /// The changes without the updates that `ignore_function` accepts; it is given the current
        /// and the previous item. The change sets left empty are dropped
        /// (`IgnoreUpdateWhen(Func<TObject, TObject, bool>)`).
        fn ignore_update_when(
            &self,
            ignore_function: impl Fn(&T, &T) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            let selected = ferroui_reactive::ObservableExt::select(&self.change_sets(), move |updates: crate::cache::ChangeSet<T, K>| {
                updates
                    .iter()
                    .filter(|update| {
                        if update.reason() != crate::cache::ChangeReason::Update {
                            return true;
                        }
                        !ignore_function(update.current(), update.previous().expect("an update has a previous item"))
                    })
                    .cloned()
                    .collect::<crate::cache::ChangeSet<T, K>>()
            });
            crate::cache::ObservableCacheEx::<T, K>::not_empty(&selected)
        }
    };
}
pub(crate) use ignore_update_when_methods;
