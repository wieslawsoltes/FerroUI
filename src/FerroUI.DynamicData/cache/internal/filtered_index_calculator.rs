//! `FilteredIndexCalculator<TObject, TKey>` (`Cache/Internal/FilteredIndexCalculator.cs`).

use super::sort_extensions::{binary_search_range, get_insert_position_linear};
use super::{KeyComparer, KeyValueCollection};
use crate::cache::{Change, ChangeReason, ChangeSet, SortReason};
use crate::list::SortException;
use std::collections::HashSet;
use std::hash::Hash;

/// The changes, with their indexes, that turn the window `previous_items` of a sorted collection
/// into `current_items` (`FilteredIndexCalculator<TObject, TKey>.Calculate`), given the changes
/// `source_updates` of the sorted collection.
pub(crate) fn calculate<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    current_items: &KeyValueCollection<T, K>,
    previous_items: &KeyValueCollection<T, K>,
    source_updates: Option<&ChangeSet<T, K>>,
) -> Result<Vec<Change<T, K>>, SortException> {
    if current_items.sort_reason() == SortReason::ComparerChanged || current_items.sort_reason() == SortReason::InitialLoad {
        // Clear the collection and rebuild it.
        let removed = previous_items.iter().enumerate().map(|(index, (key, value))| Change::with_index(ChangeReason::Remove, key.clone(), value.clone(), Some(index)));
        let new_items = current_items.iter().enumerate().map(|(index, (key, value))| Change::with_index(ChangeReason::Add, key.clone(), value.clone(), Some(index)));
        return Ok(removed.chain(new_items).collect());
    }

    let comparer = current_items.comparer().clone();
    let compare = move |x: &(K, T), y: &(K, T)| comparer.compare(x, y);
    let mut previous_list = previous_items.to_vec();
    let removes = KeyComparer::except(previous_items, current_items);
    let adds = KeyComparer::except(current_items, previous_items);
    let in_both_keys: HashSet<K> = KeyComparer::intersect(previous_items, current_items).into_iter().map(|(key, _)| key).collect();
    let mut result = Vec::new();

    for remove in removes {
        let index = KeyComparer::index_of(&previous_list, &remove).expect("a removed pair is in the previous items");
        previous_list.remove(index);
        result.push(Change::with_index(ChangeReason::Remove, remove.0, remove.1, Some(index)));
    }

    for add in adds {
        // Find the new insert position.
        let index = binary_search_range(&previous_list, 0, previous_list.len(), &add, &compare);
        let insert_index = (!index) as usize;
        result.push(Change::with_index(ChangeReason::Add, add.0.clone(), add.1.clone(), Some(insert_index)));
        previous_list.insert(insert_index, add);
    }

    // The adds and the removes are accounted for, so check whether anything in the remaining
    // changes was moved or updated.
    let remaining_items: Vec<Change<T, K>> = source_updates
        .map(|updates| {
            updates
                .iter()
                .filter(|u| in_both_keys.contains(u.key()) && matches!(u.reason(), ChangeReason::Update | ChangeReason::Moved | ChangeReason::Refresh))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    for change in &remaining_items {
        match change.reason() {
            ChangeReason::Update => {
                let previous_value = change.previous().expect("an update has a previous item").clone();
                let current = (change.key().clone(), change.current().clone());
                let previous = (change.key().clone(), previous_value.clone());

                // Remove from the actual index.
                let remove_index = KeyComparer::index_of(&previous_list, &previous).expect("an updated pair is in the previous items");
                previous_list.remove(remove_index);

                // Insert at the desired index.
                let desired_index = binary_search_range(&previous_list, 0, previous_list.len(), &current, &compare);
                let insert_index = (!desired_index) as usize;
                previous_list.insert(insert_index, current);
                result.push(Change::with_previous(
                    ChangeReason::Update,
                    change.key().clone(),
                    change.current().clone(),
                    Some(previous_value),
                    Some(insert_index),
                    Some(remove_index),
                ));
            }
            ChangeReason::Moved => {
                let current = (change.key().clone(), change.current().clone());
                let previous_index = KeyComparer::index_of(&previous_list, &current);
                let desired_index = KeyComparer::index_of(current_items, &current);
                if previous_index == desired_index {
                    continue;
                }
                let Some(desired_index) = desired_index else {
                    return Err(SortException::new("Cannot determine current index"));
                };
                let previous_index = previous_index.expect("a moved pair is in the previous items");
                previous_list.remove(previous_index);
                previous_list.insert(desired_index, current);
                result.push(Change::moved(change.key().clone(), change.current().clone(), desired_index, previous_index));
            }
            _ => result.push(change.clone()),
        }
    }

    // The alternative to evaluating is to check the order.
    let mut evaluates: Vec<&Change<T, K>> = remaining_items.iter().filter(|c| c.reason() == ChangeReason::Refresh).collect();
    evaluates.sort_by(|x, y| compare(&(y.key().clone(), y.current().clone()), &(x.key().clone(), x.current().clone())));

    // Calculate the moves; a very expensive operation.
    for u in evaluates {
        let current = (u.key().clone(), u.current().clone());
        let Some(old) = KeyComparer::index_of(&previous_list, &current) else {
            continue;
        };
        let mut new_position = get_insert_position_linear(&previous_list, &current, &compare);
        if old < new_position {
            new_position -= 1;
        }
        if old == new_position {
            continue;
        }
        previous_list.remove(old);
        previous_list.insert(new_position, current);
        result.push(Change::moved(u.key().clone(), u.current().clone(), new_position, old));
    }

    Ok(result)
}
