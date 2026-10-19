//! `TreatMovesAsRemoveAdd` (`Cache/ObservableCacheEx.TreatMovesAsRemoveAdd.cs`).

macro_rules! treat_moves_as_remove_add_methods {
    () => {
        /// The sorted change sets with each move replaced by a removal at the previous index and an
        /// addition at the current one (`TreatMovesAsRemoveAdd`).
        fn treat_moves_as_remove_add(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::SortedChangeSet<T, K>>> {
            ferroui_reactive::ObservableExt::select(&self.sorted_change_sets(), |changes: crate::cache::SortedChangeSet<T, K>| {
                let mut replaced = std::vec::Vec::with_capacity(changes.count());
                for change in changes.changes() {
                    if change.reason() == crate::cache::ChangeReason::Moved {
                        replaced.push(crate::cache::Change::with_index(
                            crate::cache::ChangeReason::Remove,
                            change.key().clone(),
                            change.current().clone(),
                            change.previous_index(),
                        ));
                        replaced.push(crate::cache::Change::with_index(
                            crate::cache::ChangeReason::Add,
                            change.key().clone(),
                            change.current().clone(),
                            change.current_index(),
                        ));
                    } else {
                        replaced.push(change.clone());
                    }
                }
                crate::cache::SortedChangeSet::new(changes.sorted_items().clone(), crate::cache::ChangeSet::from(replaced))
            })
        }
    };
}
pub(crate) use treat_moves_as_remove_add_methods;
