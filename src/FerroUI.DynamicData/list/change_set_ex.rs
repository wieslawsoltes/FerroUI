//! `ChangeSetEx` (`List/ChangeSetEx.cs`, with `List/Linq/ItemChangeEnumerator.cs` and
//! `List/Linq/WithoutIndexEnumerator.cs`): the helpers of the change sets of lists.
//!
//! `GetChangeType` is [`ListChangeReason::change_type`](super::ListChangeReason::change_type), and
//! the internal `Unified` is `list::internal::unified`.

use super::{Change, ChangeSet, ChangeType, ItemChange, ListChangeReason};

/// The changes of the change set as changes of one item each (`Flatten`, `ItemChangeEnumerator`):
/// an added range becomes additions and a removed range or a clear removals, numbered from the
/// index of the range, or from the last index known when the range has none.
pub fn flatten<T: Clone>(source: &ChangeSet<T>) -> Vec<ItemChange<T>> {
    // Upstream's indexes are `int`s, `-1` for none; the arithmetic is kept on them.
    let to_index = |index: isize| (index >= 0).then_some(index as usize);
    let mut last_known_index: isize = 0;
    let mut result = Vec::new();
    for change in source {
        if change.change_type() == ChangeType::Item {
            let item = change.item();
            last_known_index = item.current_index().map_or(-1, |index| index as isize);
            result.push(ItemChange::new(change.reason(), item.current().clone(), item.previous().cloned(), item.current_index(), item.previous_index()));
        } else {
            let mut index = change.range().index().map_or(last_known_index, |index| index as isize);
            for item in change.range().iter() {
                let reason = match change.reason() {
                    ListChangeReason::AddRange => ListChangeReason::Add,
                    ListChangeReason::RemoveRange | ListChangeReason::Clear => ListChangeReason::Remove,
                    _ => return result,
                };
                result.push(ItemChange::new(reason, item.clone(), None, to_index(index), None));
                index += 1;
                last_known_index = index;
            }
        }
    }
    result
}

/// The change set with each item converted by `transformer`; the reasons and indexes stay
/// (`Transform(IChangeSet<TSource>, Func<TSource, TDestination>)`).
pub fn transform_change_set<T, U>(source: &ChangeSet<T>, transformer: &dyn Fn(&T) -> U) -> ChangeSet<U> {
    source
        .iter()
        .map(|change| match change.try_item() {
            Some(item) => Change::with_previous(change.reason(), transformer(item.current()), item.previous().map(transformer), item.current_index(), item.previous_index()),
            None => Change::range_change(change.reason(), change.range().iter().map(transformer).collect(), change.range().index()),
        })
        .collect()
}

/// The changes without their indexes; moves, which make no sense without them, are left out
/// (`YieldWithoutIndex`, `WithoutIndexEnumerator`).
///
/// # Panics
/// For a refresh, which needs its index (`ArgumentException`, as upstream's).
pub fn yield_without_index<T: Clone>(source: &[Change<T>]) -> Vec<Change<T>> {
    source
        .iter()
        .filter(|change| change.reason() != ListChangeReason::Moved)
        .map(|change| match change.try_item() {
            Some(item) => Change::with_previous(change.reason(), item.current().clone(), item.previous().cloned(), None, None),
            None => Change::range_change(change.reason(), change.range().items().to_vec(), None),
        })
        .collect()
}
