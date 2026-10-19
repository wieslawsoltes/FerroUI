//! `CacheEx` (`Cache/Internal/CacheEx.cs`).
//!
//! `GetInitialUpdates(ChangeAwareCache)` is `ObservableCache::get_initial_updates` and the snapshot
//! of `LockFreeObservableCache`; `DictionaryExtensions.GetOrEmpty`
//! (`Cache/Internal/DictionaryExtensions.cs`) is `HashMap::get(..).cloned().unwrap_or_default()`.

use crate::cache::{ChangeReason, ChangeSet};
use std::collections::HashMap;
use std::hash::Hash;

/// Applies the adds, updates and removes of `changes` to `source` (`Clone(IDictionary, IChangeSet)`).
#[allow(dead_code)]
pub(crate) fn clone_into_dictionary<T: Clone, K: Eq + Hash + Clone>(source: &mut HashMap<K, T>, changes: &ChangeSet<T, K>) {
    for item in changes {
        match item.reason() {
            ChangeReason::Update | ChangeReason::Add => {
                source.insert(item.key().clone(), item.current().clone());
            }
            ChangeReason::Remove => {
                source.remove(item.key());
            }
            ChangeReason::Refresh | ChangeReason::Moved => {}
        }
    }
}
