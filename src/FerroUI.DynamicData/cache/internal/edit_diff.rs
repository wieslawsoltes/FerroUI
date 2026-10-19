//! `EditDiff<TObject, TKey>` (`Cache/Internal/EditDiff.cs`): the edit that makes a source cache
//! hold a new set of items, with the fewest changes.

use crate::cache::{ISourceCache, ISourceUpdater};
use std::collections::HashSet;
use std::hash::Hash;

/// `EditDiff.Edit`: removes the items whose keys are not among the new items, adds those whose
/// keys are new, and updates those that `are_equal` says differ.
pub(crate) fn edit_diff<T: Clone + 'static, K: Eq + Hash + Clone + 'static, S: ISourceCache<T, K> + ?Sized>(source: &S, items: Vec<T>, are_equal: &dyn Fn(&T, &T) -> bool) {
    let mut items = Some(items);
    source.edit(&mut |inner_cache: &mut dyn ISourceUpdater<T, K>| {
        let Some(items) = items.take() else { return };
        let original_items = inner_cache.key_values();
        let new_items = inner_cache.get_key_values(&mut items.into_iter());

        // `Except` by key: the distinct keys of the first that the second does not have.
        let new_keys: HashSet<K> = new_items.iter().map(|(key, _)| key.clone()).collect();
        let original_keys: HashSet<K> = original_items.iter().map(|(key, _)| key.clone()).collect();
        let removes: Vec<K> = original_items.iter().filter(|(key, _)| !new_keys.contains(key)).map(|(key, _)| key.clone()).collect();
        let mut seen: HashSet<K> = HashSet::new();
        let adds: Vec<(K, T)> = new_items.iter().filter(|(key, _)| !original_keys.contains(key) && seen.insert(key.clone())).cloned().collect();

        let intersect: Vec<(K, T)> = new_items
            .iter()
            .filter(|(key, item)| inner_cache.lookup(key).is_some_and(|original| !are_equal(&original, item)))
            .cloned()
            .collect();

        // `Union` by key: the distinct keys of the adds, then of the changed items.
        let mut seen: HashSet<K> = HashSet::new();
        let updates: Vec<(K, T)> = adds.into_iter().chain(intersect).filter(|(key, _)| seen.insert(key.clone())).collect();

        inner_cache.remove_keys(&mut removes.into_iter());
        inner_cache.add_or_update_pairs(&mut updates.into_iter());
    });
}
