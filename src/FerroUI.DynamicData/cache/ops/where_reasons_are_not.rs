//! `WhereReasonsAreNot` (`Cache/ObservableCacheEx.WhereReasonsAreNot.cs`).

macro_rules! where_reasons_are_not_methods {
    () => {
        /// The changes of reasons other than the given ones; the change sets left empty are
        /// dropped (`WhereReasonsAreNot(params ChangeReason[])`).
        ///
        /// # Panics
        /// When `reasons` is empty (`ArgumentException`: "Must select at least one reason").
        fn where_reasons_are_not(&self, reasons: &[crate::cache::ChangeReason]) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            assert!(!reasons.is_empty(), "Must select at least one reason");
            let hashed: std::collections::HashSet<crate::cache::ChangeReason> = reasons.iter().copied().collect();
            let selected = ferroui_reactive::ObservableExt::select(&self.change_sets(), move |updates: crate::cache::ChangeSet<T, K>| {
                updates.iter().filter(|update| !hashed.contains(&update.reason())).cloned().collect::<crate::cache::ChangeSet<T, K>>()
            });
            crate::cache::ObservableCacheEx::<T, K>::not_empty(&selected)
        }
    };
}
pub(crate) use where_reasons_are_not_methods;
