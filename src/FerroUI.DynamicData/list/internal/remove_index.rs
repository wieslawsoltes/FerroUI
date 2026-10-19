//! `RemoveIndex` (`List/ObservableListEx.RemoveIndex.cs`, `List/Linq/WithoutIndexEnumerator.cs`),
//! which the merging operators apply inside: the changes without their indexes, and without the
//! moves, which make no sense without them.

use crate::list::{Change, ChangeSet, ChangeType, ListChangeReason};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The change sets of `source` without indexes.
pub(crate) fn remove_index<T: Clone + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    source.select(|changes: ChangeSet<T>| without_index(&changes))
}

/// `YieldWithoutIndex`.
pub(crate) fn without_index<T: Clone>(changes: &ChangeSet<T>) -> ChangeSet<T> {
    changes
        .iter()
        .filter(|change| change.reason() != ListChangeReason::Moved)
        .map(|change| {
            if change.change_type() == ChangeType::Item {
                let item = change.item();
                // Deviation (DEVIATIONS.md, DynamicData): a refresh keeps its index, which the change
                // of a refresh needs; upstream's `Change` throws for a refresh without one.
                let index = if change.reason() == ListChangeReason::Refresh { item.current_index() } else { None };
                Change::with_previous(change.reason(), item.current().clone(), item.previous().cloned(), index, None)
            } else {
                Change::range_change(change.reason(), change.range().items().to_vec(), None)
            }
        })
        .collect()
}
