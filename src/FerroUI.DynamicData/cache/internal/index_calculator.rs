//! `IndexCalculator<TObject, TKey>` (`Cache/Internal/IndexCalculator.cs`).

use super::sort_extensions::{binary_search_range, get_insert_position_linear};
use super::{KeyComparer, KeyValueComparer};
use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, IQuery, SortOptimisations};
use crate::list::SortException;
use std::cmp::Ordering;
use std::hash::Hash;

/// Keeps the pairs of a sorted cache in order and computes the indexes of its changes
/// (`IndexCalculator<TObject, TKey>`).
pub(crate) struct IndexCalculator<T, K> {
    comparer: KeyValueComparer<T, K>,
    optimisations: SortOptimisations,
    list: Vec<(K, T)>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IndexCalculator<T, K> {
    pub(crate) fn new(comparer: KeyValueComparer<T, K>, optimisations: SortOptimisations) -> Self {
        Self { comparer, optimisations, list: Vec::new() }
    }

    /// The pairs in order (`List`).
    pub(crate) fn list(&self) -> &[(K, T)] {
        &self.list
    }

    /// Applies `changes` to the sorted pairs, and returns them with their indexes, followed by the
    /// moves that the refreshes cause (`Calculate`).
    pub(crate) fn calculate(&mut self, changes: &ChangeSet<T, K>) -> Result<ChangeSet<T, K>, SortException> {
        let mut result = Vec::with_capacity(changes.count());
        let mut refreshes = Vec::with_capacity(changes.refreshes());
        for u in changes {
            let current = (u.key().clone(), u.current().clone());
            match u.reason() {
                ChangeReason::Add => {
                    let position = self.get_insert_position_binary(&current);
                    self.list.insert(position, current);
                    result.push(Change::with_index(ChangeReason::Add, u.key().clone(), u.current().clone(), Some(position)));
                }
                ChangeReason::Update => {
                    let previous_item = u.previous().expect("an update has a previous item").clone();
                    let previous = (u.key().clone(), previous_item.clone());
                    let old = self.get_current_position(&previous)?;
                    self.list.remove(old);
                    let new_position = self.get_insert_position_binary(&current);
                    self.list.insert(new_position, current);
                    result.push(Change::with_previous(
                        ChangeReason::Update,
                        u.key().clone(),
                        u.current().clone(),
                        Some(previous_item),
                        Some(new_position),
                        Some(old),
                    ));
                }
                ChangeReason::Remove => {
                    let position = self.get_current_position(&current)?;
                    self.list.remove(position);
                    result.push(Change::with_index(ChangeReason::Remove, u.key().clone(), u.current().clone(), Some(position)));
                }
                ChangeReason::Refresh => {
                    refreshes.push(u.clone());
                    result.push(u.clone());
                }
                ChangeReason::Moved => {}
            }
        }

        // For the refreshes, check whether the change forces a new position.
        let comparer = self.comparer.clone();
        let mut evaluates = refreshes;
        evaluates.sort_by(|x, y| comparer.compare(&(y.key().clone(), y.current().clone()), &(x.key().clone(), x.current().clone())));
        if !evaluates.is_empty() && self.optimisations.has_flag(SortOptimisations::IGNORE_EVALUATES) {
            // Sort the entire sequence again, without moves.
            self.list.sort_by(|x, y| comparer.compare(x, y));
        } else {
            // Calculate the moves; a very expensive operation.
            for u in &evaluates {
                let current = (u.key().clone(), u.current().clone());
                let Some(old) = KeyComparer::index_of(&self.list, &current) else {
                    continue;
                };
                let mut new_position = get_insert_position_linear(&self.list, &current, &|x, y| comparer.compare(x, y));
                if old < new_position {
                    new_position -= 1;
                }
                if old == new_position {
                    continue;
                }
                self.list.remove(old);
                self.list.insert(new_position, current);
                result.push(Change::moved(u.key().clone(), u.current().clone(), new_position, old));
            }
        }

        Ok(ChangeSet::from(result))
    }

    /// Sorts with `comparer` from now on (`ChangeComparer`).
    pub(crate) fn change_comparer(&mut self, comparer: KeyValueComparer<T, K>) -> ChangeSet<T, K> {
        self.comparer = comparer;
        ChangeSet::empty()
    }

    /// Sorts the items of `cache` and returns them as additions with their indexes (`Load`): the
    /// first batch of changes may have arrived before the comparer was set, so it is inferred from
    /// the cache.
    pub(crate) fn load(&mut self, cache: &ChangeAwareCache<T, K>) -> ChangeSet<T, K> {
        self.reset(cache);
        self.list.iter().enumerate().map(|(index, (key, value))| Change::with_index(ChangeReason::Add, key.clone(), value.clone(), Some(index))).collect()
    }

    /// Sorts the pairs again and returns the moves (`Reorder`); with `IgnoreEvaluates`, sorts
    /// without moves.
    pub(crate) fn reorder(&mut self) -> ChangeSet<T, K> {
        let mut result = Vec::new();
        let comparer = self.comparer.clone();
        if self.optimisations.has_flag(SortOptimisations::IGNORE_EVALUATES) {
            // Sort the entire sequence again, without moves.
            self.list.sort_by(|x, y| comparer.compare(x, y));
        } else {
            let mut ordered = self.list.clone();
            ordered.sort_by(|x, y| comparer.compare(x, y));
            for (index, current) in ordered.into_iter().enumerate() {
                // A binary search cannot be used, as a new sort implies that values changed.
                if current.0 == self.list[index].0 {
                    continue;
                }
                let old = KeyComparer::index_of(&self.list, &current).expect("a sorted pair is in the list");
                self.list.remove(old);
                result.push(Change::moved(current.0.clone(), current.1.clone(), index, old));
                self.list.insert(index, current);
            }
        }
        ChangeSet::from(result)
    }

    /// Sorts the items of `cache` again (`Reset`).
    pub(crate) fn reset(&mut self, cache: &ChangeAwareCache<T, K>) {
        let comparer = self.comparer.clone();
        let mut list = cache.key_values();
        list.sort_by(|x, y| comparer.compare(x, y));
        self.list = list;
    }

    fn compare(&self, x: &(K, T), y: &(K, T)) -> Ordering {
        self.comparer.compare(x, y)
    }

    fn get_current_position(&self, item: &(K, T)) -> Result<usize, SortException> {
        if self.optimisations.has_flag(SortOptimisations::COMPARES_IMMUTABLE_VALUES_ONLY) {
            let index = binary_search_range(&self.list, 0, self.list.len(), item, &|x, y| self.compare(x, y));
            if index < 0 {
                return Err(SortException::new(
                    "Current position cannot be found.  Ensure the comparer includes a unique value, or do not specify ComparesImmutableValuesOnly",
                ));
            }
            Ok(index as usize)
        } else {
            KeyComparer::index_of(&self.list, item).ok_or_else(|| SortException::new("Current position cannot be found. The item is not in the collection"))
        }
    }

    fn get_insert_position_binary(&self, item: &(K, T)) -> usize {
        let compare = |x: &(K, T), y: &(K, T)| self.compare(x, y);
        let mut index = binary_search_range(&self.list, 0, self.list.len(), item, &compare);
        if index > 0 {
            let temp_index = index;
            index = binary_search_range(&self.list, (temp_index - 1) as usize, self.list.len() - temp_index as usize, item, &compare);
            if index > 0 {
                return temp_index as usize;
            }
        }
        (!index) as usize
    }
}
