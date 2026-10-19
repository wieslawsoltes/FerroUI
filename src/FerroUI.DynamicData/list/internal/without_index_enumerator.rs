//! `WithoutIndexEnumerator<T>` (`List/Linq/WithoutIndexEnumerator.cs`, `ChangeSetEx.YieldWithoutIndex`).

use crate::list::{Change, ChangeType, ListChangeReason};

/// The changes without their indexes; moves, which make no sense without them, are dropped
/// (`YieldWithoutIndex`).
///
/// # Panics
/// For a refresh, which upstream cannot build without an index (`ArgumentException`).
pub(crate) fn yield_without_index<'a, T: Clone + 'a>(changes: impl IntoIterator<Item = &'a Change<T>>) -> Vec<Change<T>> {
    changes
        .into_iter()
        // exceptional case - makes no sense to remove index from move
        .filter(|change| change.reason() != ListChangeReason::Moved)
        .map(|change| match change.change_type() {
            ChangeType::Item => {
                let item = change.item();
                Change::with_previous(change.reason(), item.current().clone(), item.previous().cloned(), None, None)
            }
            ChangeType::Range => Change::range_change(change.reason(), change.range().items().to_vec(), None),
        })
        .collect()
}
