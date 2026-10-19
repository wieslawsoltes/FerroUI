//! `WhereReasonsAreNot` (`List/ObservableListEx.WhereReasonsAreNot.cs`).

macro_rules! where_reasons_are_not_methods {
    () => {
        /// The changes of reasons other than the given ones, without their indexes (moves are
        /// dropped), unless only refreshes are left out, which keeps the indexes; the change sets
        /// left empty are dropped (`WhereReasonsAreNot(params ListChangeReason[])`).
        ///
        /// # Panics
        /// When `reasons` is empty (`ArgumentException`: "Must enter at least 1 reason"), and for a
        /// refresh, which cannot be without an index.
        fn where_reasons_are_not(&self, reasons: &[crate::list::ListChangeReason]) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            assert!(!reasons.is_empty(), "Must enter at least 1 reason");
            if reasons.len() == 1 && reasons[0] == crate::list::ListChangeReason::Refresh {
                // If only refresh changes are removed, then there's no need to remove the indexes
                let selected = ferroui_reactive::ObservableExt::select(&self.change_sets(), |changes: crate::list::ChangeSet<T>| {
                    changes.iter().filter(|change| change.reason() != crate::list::ListChangeReason::Refresh).cloned().collect::<crate::list::ChangeSet<T>>()
                });
                return crate::list::ObservableListEx::<T>::not_empty(&selected);
            }
            let matches: std::collections::HashSet<crate::list::ListChangeReason> = reasons.iter().copied().collect();
            let selected = ferroui_reactive::ObservableExt::select(&self.change_sets(), move |updates: crate::list::ChangeSet<T>| {
                crate::list::ChangeSet::from(crate::list::internal::yield_without_index(updates.iter().filter(|update| !matches.contains(&update.reason()))))
            });
            crate::list::ObservableListEx::<T>::not_empty(&selected)
        }
    };
}
pub(crate) use where_reasons_are_not_methods;
