//! `WhereReasonsAre` (`List/ObservableListEx.WhereReasonsAre.cs`).

macro_rules! where_reasons_are_methods {
    () => {
        /// The changes of the given reasons, without their indexes (moves are dropped); the change
        /// sets left empty are dropped (`WhereReasonsAre(params ListChangeReason[])`).
        ///
        /// # Panics
        /// When `reasons` is empty (`ArgumentException`: "Must enter at least 1 reason"), and for a
        /// refresh, which cannot be without an index.
        fn where_reasons_are(&self, reasons: &[crate::list::ListChangeReason]) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            assert!(!reasons.is_empty(), "Must enter at least 1 reason");
            let matches: std::collections::HashSet<crate::list::ListChangeReason> = reasons.iter().copied().collect();
            let selected = ferroui_reactive::ObservableExt::select(&self.change_sets(), move |changes: crate::list::ChangeSet<T>| {
                crate::list::ChangeSet::from(crate::list::internal::yield_without_index(changes.iter().filter(|change| matches.contains(&change.reason()))))
            });
            crate::list::ObservableListEx::<T>::not_empty(&selected)
        }
    };
}
pub(crate) use where_reasons_are_methods;
