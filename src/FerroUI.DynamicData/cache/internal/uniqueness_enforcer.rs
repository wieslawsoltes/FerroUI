//! `UniquenessEnforcer<TObject, TKey>` (`Cache/Internal/UniquenessEnforcer.cs`).

use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, ICache};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

/// `UniquenessEnforcer.Run`: one change per key in each change set (the last that is not a
/// refresh, when there are several; for a refresh, an earlier add or update of the batch is used),
/// applied to a cache whose changes are the result; moves are left out.
pub(crate) fn uniqueness_enforcer<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(source: Rc<dyn IObservable<ChangeSet<T, K>>>) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::defer(move || {
        let cache: RefCell<Option<ChangeAwareCache<T, K>>> = RefCell::new(None);
        source
            .where_(|changes: &ChangeSet<T, K>| changes.iter().any(|change| change.reason() != ChangeReason::Moved))
            .select(|changes: ChangeSet<T, K>| -> ChangeSet<T, K> { changes.iter().filter(|change| change.reason() != ChangeReason::Moved).cloned().collect() })
            .select(move |changes: ChangeSet<T, K>| {
                let mut cache = cache.borrow_mut();
                let cache = cache.get_or_insert_with(|| ChangeAwareCache::with_capacity(changes.count()));

                // `GroupBy` of the key, in the order of the first change of each key.
                let mut groups: Vec<Vec<&Change<T, K>>> = Vec::new();
                let mut group_of_key: HashMap<&K, usize> = HashMap::new();
                for change in &changes {
                    match group_of_key.get(change.key()) {
                        Some(&group) => groups[group].push(change),
                        None => {
                            group_of_key.insert(change.key(), groups.len());
                            groups.push(vec![change]);
                        }
                    }
                }

                for all in groups {
                    // The extreme edge case of a compound of changes that ends with a refresh.
                    let change = if all.len() > 1 { all.iter().rev().find(|candidate| candidate.reason() != ChangeReason::Refresh).copied().unwrap_or(all[0]) } else { all[0] };
                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => cache.add_or_update(change.current().clone(), change.key().clone()),
                        ChangeReason::Remove => cache.remove(change.key()),
                        ChangeReason::Refresh => cache.refresh(change.key()),
                        ChangeReason::Moved => {}
                    }
                }
                cache.capture_changes()
            })
    })
}
