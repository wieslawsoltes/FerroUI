//! `AddKeyEnumerator<TObject, TKey>` (`List/Linq/AddKeyEnumerator.cs`): the changes of a list as
//! changes of a cache.

use crate::cache::{Change as CacheChange, ChangeReason, ChangeSet as CacheChangeSet};
use crate::list::{ChangeSet, ListChangeReason};

/// The changes of `changes` keyed by `key_selector`: a replacement whose key is unchanged is an
/// update, another a removal and an addition.
///
/// # Panics
/// For a refresh (`IndexOutOfRangeException`: "The changes are not of a supported type.").
pub(crate) fn add_key<T: Clone, K: PartialEq + Clone>(changes: &ChangeSet<T>, key_selector: &dyn Fn(&T) -> K) -> CacheChangeSet<T, K> {
    let mut result = Vec::new();
    for change in changes {
        match change.reason() {
            ListChangeReason::Add => {
                let item = change.item().current();
                result.push(CacheChange::new(ChangeReason::Add, key_selector(item), item.clone()));
            }
            ListChangeReason::AddRange => {
                result.extend(change.range().iter().map(|item| CacheChange::new(ChangeReason::Add, key_selector(item), item.clone())));
            }
            ListChangeReason::Replace => {
                // A replacement is a removal and an addition, if and only if the keys differ.
                let previous = change.item().previous().expect("a replacement has a previous item");
                let current = change.item().current();
                let (previous_key, current_key) = (key_selector(previous), key_selector(current));
                if current_key == previous_key {
                    result.push(CacheChange::update(current_key, current.clone(), previous.clone()));
                } else {
                    result.push(CacheChange::new(ChangeReason::Remove, previous_key, previous.clone()));
                    result.push(CacheChange::new(ChangeReason::Add, current_key, current.clone()));
                }
            }
            ListChangeReason::Remove => {
                let item = change.item().current();
                result.push(CacheChange::new(ChangeReason::Remove, key_selector(item), item.clone()));
            }
            ListChangeReason::Clear | ListChangeReason::RemoveRange => {
                result.extend(change.range().iter().map(|item| CacheChange::new(ChangeReason::Remove, key_selector(item), item.clone())));
            }
            ListChangeReason::Moved => {
                let item = change.item();
                result.push(CacheChange::with_previous(
                    ChangeReason::Moved,
                    key_selector(item.current()),
                    item.current().clone(),
                    item.previous().cloned(),
                    item.current_index(),
                    item.previous_index(),
                ));
            }
            ListChangeReason::Refresh => panic!("The changes are not of a supported type."),
        }
    }
    CacheChangeSet::from(result)
}
