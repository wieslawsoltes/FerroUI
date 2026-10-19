//! `ItemChangeEnumerator<T>` (`List/Linq/ItemChangeEnumerator.cs`, `ChangeSetEx.Flatten`): the
//! changes of a change set as the changes of one item each.

use crate::list::{ChangeSet, ChangeType, ItemChange, ListChangeReason};

/// The change of each item (`Flatten`): an `AddRange` spread into adds, a `RemoveRange` and a
/// `Clear` into removes, each with the index of its item; a range without an index starts at the
/// last index known.
pub(crate) fn flatten_item_changes<T: Clone>(changes: &ChangeSet<T>) -> Vec<ItemChange<T>> {
    // Upstream's indexes, where -1 is none.
    let to_index = |index: Option<usize>| index.map_or(-1, |index| index as isize);
    let from_index = |index: isize| if index < 0 { None } else { Some(index as usize) };
    let mut result = Vec::new();
    let mut last_known_index: isize = 0;
    for change in changes {
        if change.change_type() == ChangeType::Item {
            let item = change.item();
            last_known_index = to_index(item.current_index());
            result.push(item.clone());
            continue;
        }
        let range = change.range();
        let mut index = if range.index().is_none() { last_known_index } else { to_index(range.index()) };
        for item in range {
            let reason = match change.reason() {
                ListChangeReason::AddRange => ListChangeReason::Add,
                ListChangeReason::RemoveRange | ListChangeReason::Clear => ListChangeReason::Remove,
                _ => return result,
            };
            result.push(ItemChange::new(reason, item.clone(), None, from_index(index), None));
            index += 1;
            last_known_index = index;
        }
    }
    result
}
