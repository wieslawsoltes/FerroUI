//! `UnifiedChange<T>` and `UnifiedChangeEnumerator<T>` (`List/Internal/UnifiedChange.cs`,
//! `List/Linq/UnifiedChangeEnumerator.cs`): the changes of a change set as changes of one item
//! each.

use crate::list::{ChangeSet, ChangeType, ListChangeReason};

/// The change of one item, a range spread out (`UnifiedChange<T>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnifiedChange<T> {
    pub(crate) reason: ListChangeReason,
    pub(crate) current: T,
    pub(crate) previous: Option<T>,
}

/// The changes of `changes`, a range spread into the changes of its items (`Unified`): an
/// `AddRange` into adds, a `RemoveRange` into removes, a `Clear` into clears of each item.
pub(crate) fn unified<T: Clone>(changes: &ChangeSet<T>) -> Vec<UnifiedChange<T>> {
    let mut result = Vec::new();
    for change in changes {
        if change.change_type() == ChangeType::Item {
            let item = change.item();
            result.push(UnifiedChange { reason: change.reason(), current: item.current().clone(), previous: item.previous().cloned() });
            continue;
        }
        let reason = match change.reason() {
            ListChangeReason::AddRange => ListChangeReason::Add,
            ListChangeReason::RemoveRange => ListChangeReason::Remove,
            ListChangeReason::Clear => ListChangeReason::Clear,
            _ => continue,
        };
        result.extend(change.range().iter().map(|item| UnifiedChange { reason, current: item.clone(), previous: None }));
    }
    result
}
