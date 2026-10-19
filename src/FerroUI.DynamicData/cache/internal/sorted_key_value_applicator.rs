//! `SortedKeyValueApplicator<TObject, TKey>` (`Cache/Internal/SortedKeyValueApplicator.cs`):
//! keeps a sorted list of the pairs of a key and an item; used by `SortAndPage` and
//! `SortAndVirtualize`.

use super::sort_extensions::{get_current_position, get_insert_position, get_insert_position_linear};
use super::{Cache, KeyComparer, KeyValueComparer};
use crate::cache::{ChangeReason, ChangeSet, ICache, IQuery};
use crate::list::SortException;
use std::hash::Hash;

/// The options of the applicator (the parts of `SortAndBindOptions` it reads).
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream passes a `SortAndBindOptions`, of which the
/// applicator reads these two parts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ApplicatorOptions {
    /// The number of changes above which the list is rebuilt; 0 never rebuilds (`ResetThreshold`).
    pub(crate) reset_threshold: usize,
    /// Find positions with a binary search (`UseBinarySearch`).
    pub(crate) use_binary_search: bool,
}

/// Keeps `target` sorted as the changes of a cache arrive (`SortedKeyValueApplicator<TObject,
/// TKey>`).
pub(crate) struct SortedKeyValueApplicator<T, K> {
    cache: Cache<T, K>,
    target: Vec<(K, T)>,
    options: ApplicatorOptions,
    comparer: KeyValueComparer<T, K>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> SortedKeyValueApplicator<T, K> {
    pub(crate) fn new(target: Vec<(K, T)>, comparer: KeyValueComparer<T, K>, options: ApplicatorOptions) -> Self {
        Self { cache: Cache::new(), target, options, comparer }
    }

    /// The sorted pairs.
    pub(crate) fn target(&self) -> &[(K, T)] {
        &self.target
    }

    /// Sorts with `comparer` from now on, and sorts the list again (`ChangeComparer`).
    pub(crate) fn change_comparer(&mut self, comparer: KeyValueComparer<T, K>) {
        self.comparer = comparer;
        let comparer = self.comparer.clone();
        self.target.sort_by(|x, y| comparer.compare(x, y));
    }

    /// Applies `changes` to the list, or rebuilds it when there are more changes than the reset
    /// threshold (`ProcessChanges`).
    pub(crate) fn process_changes(&mut self, changes: &ChangeSet<T, K>) -> Result<(), SortException> {
        self.cache.clone_changes(changes);
        let fire_reset = self.options.reset_threshold > 0 && self.options.reset_threshold < changes.count();
        if fire_reset {
            self.reset();
            Ok(())
        } else {
            self.apply_changes(changes)
        }
    }

    /// Rebuilds the list from the items (`Reset`).
    pub(crate) fn reset(&mut self) {
        let comparer = self.comparer.clone();
        let mut sorted = self.cache.key_values();
        sorted.sort_by(|x, y| comparer.compare(x, y));
        self.target = sorted;
    }

    fn apply_changes(&mut self, changes: &ChangeSet<T, K>) -> Result<(), SortException> {
        // Iterate through the changes, find the sorted positions and apply the changes.
        for change in changes {
            let item = (change.key().clone(), change.current().clone());
            match change.reason() {
                ChangeReason::Add => {
                    let index = self.get_insert_position(&item);
                    self.target.insert(index, item);
                }
                ChangeReason::Update => {
                    let previous = (change.key().clone(), change.previous().expect("an update has a previous item").clone());
                    let current_index = self.get_current_position(&previous)?;
                    self.target.remove(current_index);
                    let updated_index = self.get_insert_position(&item);
                    self.target.insert(updated_index, item);
                }
                ChangeReason::Remove => {
                    let current_index = self.get_current_position(&item)?;
                    self.target.remove(current_index);
                }
                ChangeReason::Refresh => {
                    // Look up the current and the new location, with the linear methods, as a
                    // binary search does not work on a list that may not be sorted any more.
                    let current_index = KeyComparer::index_of(&self.target, &item).ok_or_else(|| {
                        SortException::new(format!("Cannot find item: {} from {} items", std::any::type_name::<(K, T)>(), self.target.len()))
                    })?;
                    let comparer = self.comparer.clone();
                    let mut updated_index = get_insert_position_linear(&self.target, &item, &|x, y| comparer.compare(x, y));
                    // Recalibrate, as the insert position counts the current item.
                    if current_index < updated_index {
                        updated_index -= 1;
                    }
                    if updated_index != current_index {
                        self.target.remove(current_index);
                        self.target.insert(updated_index, item);
                    }
                }
                ChangeReason::Moved => {}
            }
        }
        Ok(())
    }

    fn get_current_position(&self, item: &(K, T)) -> Result<usize, SortException> {
        get_current_position(&self.target, item, &|x, y| self.comparer.compare(x, y), self.options.use_binary_search, &KeyComparer::equals::<T, K>)
    }

    fn get_insert_position(&self, item: &(K, T)) -> usize {
        get_insert_position(&self.target, item, &|x, y| self.comparer.compare(x, y), self.options.use_binary_search)
    }
}
