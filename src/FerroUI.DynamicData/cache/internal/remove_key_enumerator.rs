//! `RemoveKeyEnumerator<TObject, TKey>` (`Cache/Internal/RemoveKeyEnumerator.cs`): the changes of a
//! cache as changes of a list.

use crate::cache::{ChangeReason, ChangeSet};
use crate::list::{self, ListChangeReason};

/// The changes without their keys: an update becomes a removal and an addition, and a refresh,
/// without the list to find the item in, a replacement of the item by itself.
pub(crate) fn remove_key_changes<T: Clone + PartialEq, K>(source: &ChangeSet<T, K>, list: Option<&dyn list::IList<T>>) -> Vec<list::Change<T>> {
    let mut result = Vec::with_capacity(source.count());
    for change in source {
        match change.reason() {
            ChangeReason::Add => result.push(list::Change::new(ListChangeReason::Add, change.current().clone(), change.current_index())),
            ChangeReason::Refresh => match list.and_then(|list| list.index_of(change.current())) {
                Some(index) => result.push(list::Change::new(ListChangeReason::Refresh, change.current().clone(), Some(index))),
                None => result.push(list::Change::with_previous(ListChangeReason::Replace, change.current().clone(), Some(change.current().clone()), None, None)),
            },
            ChangeReason::Moved => result.push(list::Change::moved(
                change.current().clone(),
                change.current_index().expect("a move has its index"),
                change.previous_index().expect("a move has its previous index"),
            )),
            ChangeReason::Update => {
                let previous = change.previous().expect("an update has its previous item").clone();
                result.push(list::Change::new(ListChangeReason::Remove, previous, change.previous_index()));
                result.push(list::Change::new(ListChangeReason::Add, change.current().clone(), change.current_index()));
            }
            ChangeReason::Remove => result.push(list::Change::new(ListChangeReason::Remove, change.current().clone(), change.current_index())),
        }
    }
    result
}
