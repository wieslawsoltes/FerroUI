//! `ExpirableItem<TObject, TKey>` (`Cache/Internal/ExpirableItem.cs`).

use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache};
use std::cell::Cell;
use std::fmt;
use std::hash::Hash;
use std::time::SystemTime;

/// An item, its key, when it was added and its order of addition (`ExpirableItem<TObject,
/// TKey>`): equal by the key and the time, as upstream's.
#[derive(Clone)]
pub(crate) struct ExpirableItem<T, K> {
    pub(crate) value: T,
    pub(crate) key: K,
    pub(crate) expire_at: SystemTime,
    pub(crate) index: u64,
}

impl<T, K: PartialEq> PartialEq for ExpirableItem<T, K> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.expire_at == other.expire_at
    }
}

impl<T, K: fmt::Display> fmt::Display for ExpirableItem<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key: {}, Expire At: {:?}", self.key, self.expire_at)
    }
}

/// The `Transform` of the size limiters: each added or updated item becomes an expirable item
/// stamped now and numbered from `next_index`; a refresh keeps the item, and moves are ignored.
pub(crate) fn transform_to_expirable<T: Clone, K: Eq + Hash + Clone>(
    cache: &mut ChangeAwareCache<ExpirableItem<T, K>, K>,
    changes: &ChangeSet<T, K>,
    next_index: &Cell<u64>,
) -> ChangeSet<ExpirableItem<T, K>, K> {
    for change in changes {
        match change.reason() {
            ChangeReason::Add | ChangeReason::Update => {
                let index = next_index.get();
                next_index.set(index + 1);
                let item = ExpirableItem { value: change.current().clone(), key: change.key().clone(), expire_at: SystemTime::now(), index };
                cache.add_or_update(item, change.key().clone());
            }
            ChangeReason::Remove => cache.remove(change.key()),
            ChangeReason::Refresh => cache.refresh(change.key()),
            ChangeReason::Moved => {}
        }
    }
    cache.capture_changes()
}
