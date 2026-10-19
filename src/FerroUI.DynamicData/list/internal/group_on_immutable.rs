//! `GroupOnImmutable<TObject, TGroupKey>` (`List/Internal/GroupOnImmutable.cs`).

use super::group_on::{group_by, items_with_group_key, to_collection, ItemWithGroupKey};
use super::unified;
use crate::list::{replace, ChangeAwareList, ChangeSet, IGrouping, ImmutableGroup, ListChangeReason};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

/// The state of a subscription to `GroupOnImmutable`: the states of the groups as a list, and the
/// items of each group by group key (`GroupContainer`).
struct GroupState<T, G> {
    groupings: ChangeAwareList<ImmutableGroup<T, G>>,
    group_cache: HashMap<G, Vec<T>>,
}

/// The groups of the items of `source` by `group_selector`, as states that do not change,
/// evaluated again for every item when `regrouper` produces a value (`GroupOnImmutable.Run`).
pub(crate) fn group_on_immutable<T, G>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    group_selector: Rc<dyn Fn(&T) -> G>,
    regrouper: Option<Rc<dyn IObservable<()>>>,
) -> Rc<dyn IObservable<ChangeSet<ImmutableGroup<T, G>>>>
where
    T: Clone + PartialEq + 'static,
    G: Eq + Hash + Clone + 'static,
{
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<ImmutableGroup<T, G>>>>| {
        let state = Rc::new(RefCell::new(GroupState { groupings: ChangeAwareList::new(), group_cache: HashMap::new() }));

        // The group key of each item is selected once, up front.
        let items_with_group = items_with_group_key(source.clone(), group_selector.clone());

        // The `lock` of upstream serializes the source and the regrouper; on one thread they are
        // already serial.
        let shared = items_with_group.publish();
        let shared_items: Rc<dyn IObservable<ChangeSet<ItemWithGroupKey<T, G>>>> = shared.clone();

        let processing = state.clone();
        let grouper = shared_items.select(move |changes: ChangeSet<ItemWithGroupKey<T, G>>| process(&processing, &changes));

        let regroup_func: Rc<dyn IObservable<ChangeSet<ImmutableGroup<T, G>>>> = match &regrouper {
            None => Observable::never(),
            Some(regrouper) => {
                let (regrouping, selector) = (state.clone(), group_selector.clone());
                regrouper.combine_latest(to_collection(shared_items.clone()), move |_: (), collection: Rc<Vec<ItemWithGroupKey<T, G>>>| {
                    regroup(&regrouping, &*selector, &collection)
                })
            }
        };

        let publisher = grouper.merge(regroup_func).where_(|changes: &ChangeSet<ImmutableGroup<T, G>>| changes.count() != 0).subscribe(observer);

        let connected = shared.connect();
        Disposable::create(move || {
            publisher.dispose();
            connected.dispose();
        })
    })
}

/// The state of the group `key` (`GetGroupState`).
fn group_state<T: Clone, G: Clone>(key: &G, list: &[T]) -> ImmutableGroup<T, G> {
    ImmutableGroup::new(key.clone(), list.to_vec())
}

/// The change set of the groups whose state changed, from their states before
/// (`CreateChangeSet`).
fn create_change_set<T: Clone + PartialEq, G: Eq + Hash + Clone>(
    result: &mut ChangeAwareList<ImmutableGroup<T, G>>,
    all_groupings: &mut HashMap<G, Vec<T>>,
    initial_state_of_groups: Dictionary<G, ImmutableGroup<T, G>>,
) -> ChangeSet<ImmutableGroup<T, G>> {
    // Now maintain the target list.
    for (key, initial_group) in initial_state_of_groups.iter() {
        let current = all_groupings.get(key).expect("every group of the initial states is a grouping");
        if current.is_empty() {
            // Remove when empty.
            all_groupings.remove(key);
            result.remove(initial_group);
        } else {
            let current_state = group_state(key, current);
            if initial_group.count() == 0 {
                // An add.
                result.add(current_state);
            } else {
                // A replacement.
                replace(&mut *result, initial_group, current_state);
            }
        }
    }

    result.capture_changes()
}

/// The items of the group of `key`, made when there are none (`GetGroup`).
fn get_group<'a, T, G: Eq + Hash + Clone>(group_caches: &'a mut HashMap<G, Vec<T>>, key: &G) -> &'a mut Vec<T> {
    group_caches.entry(key.clone()).or_default()
}

/// Records the state of the group `key` before its first change.
fn capture_initial_state<T: Clone, G: Eq + Hash + Clone>(initial_state_of_groups: &mut Dictionary<G, ImmutableGroup<T, G>>, key: &G, list: &[T]) {
    if !initial_state_of_groups.contains_key(key) {
        initial_state_of_groups.insert(key.clone(), group_state(key, list));
    }
}

/// Removes the first item equal to `item` from `list` (`IList.Remove`).
fn remove_item<T: PartialEq>(list: &mut Vec<T>, item: &T) {
    if let Some(index) = list.iter().position(|candidate| candidate == item) {
        list.remove(index);
    }
}

/// Removes `item` from the group of `group_key`, recording the state of that group first.
fn remove_from_old_group<T: Clone + PartialEq, G: Eq + Hash + Clone>(
    all_groupings: &mut HashMap<G, Vec<T>>,
    initial_state_of_groups: &mut Dictionary<G, ImmutableGroup<T, G>>,
    group_key: &G,
    item: &T,
) {
    if let Some(old_group) = all_groupings.get_mut(group_key) {
        capture_initial_state(initial_state_of_groups, group_key, old_group);
        remove_item(old_group, item);
    }
}

/// `Process`.
fn process<T: Clone + PartialEq + 'static, G: Eq + Hash + Clone + 'static>(state: &RefCell<GroupState<T, G>>, changes: &ChangeSet<ItemWithGroupKey<T, G>>) -> ChangeSet<ImmutableGroup<T, G>> {
    let mut state = state.borrow_mut();
    let GroupState { groupings: result, group_cache: all_groupings } = &mut *state;

    // The groups that change, with their states before, to make the right notifications.
    let mut initial_state_of_groups: Dictionary<G, ImmutableGroup<T, G>> = Dictionary::new();

    for (current_group, grouping) in group_by(unified(changes), |change| change.current.group()) {
        // Find the group, made when there is none.
        get_group(all_groupings, &current_group);

        // Process the changes of the group.
        for change in grouping {
            match change.reason {
                ListChangeReason::Add => {
                    let list_to_modify = get_group(all_groupings, &current_group);
                    capture_initial_state(&mut initial_state_of_groups, &current_group, list_to_modify);
                    list_to_modify.push(change.current.item().clone());
                }
                ListChangeReason::Refresh => {
                    let previous_item = change.current.item().clone();
                    let previous_group = change.current.previous_group().cloned().expect("a refreshed item has the group key of the item it replaced");
                    let current_item = change.current.item().clone();

                    // Whether the change of the item moved it to another group.
                    if previous_group != current_group {
                        let list_to_modify = get_group(all_groupings, &current_group);
                        capture_initial_state(&mut initial_state_of_groups, &current_group, list_to_modify);

                        // Add to the new group, and remove from the old one.
                        list_to_modify.push(current_item);
                        remove_from_old_group(all_groupings, &mut initial_state_of_groups, &previous_group, &previous_item);
                    }
                }
                ListChangeReason::Replace => {
                    let list_to_modify = get_group(all_groupings, &current_group);
                    capture_initial_state(&mut initial_state_of_groups, &current_group, list_to_modify);
                    let previous = change.previous.as_ref().expect("a replacement has a previous item");
                    let previous_item = previous.item().clone();
                    let previous_group = previous.group();

                    // Whether the change of the item moved it to another group.
                    if previous_group == current_group {
                        // Find and replace.
                        let index = list_to_modify.iter().position(|item| *item == previous_item).expect("the replaced item is in its group");
                        list_to_modify[index] = change.current.item().clone();
                    } else {
                        // Add to the new group, and remove from the old one.
                        list_to_modify.push(change.current.item().clone());
                        remove_from_old_group(all_groupings, &mut initial_state_of_groups, &previous_group, &previous_item);
                    }
                }
                ListChangeReason::Remove => {
                    let list_to_modify = get_group(all_groupings, &current_group);
                    capture_initial_state(&mut initial_state_of_groups, &current_group, list_to_modify);
                    remove_item(list_to_modify, change.current.item());
                }
                ListChangeReason::Clear => {
                    let list_to_modify = get_group(all_groupings, &current_group);
                    capture_initial_state(&mut initial_state_of_groups, &current_group, list_to_modify);
                    list_to_modify.clear();
                }
                ListChangeReason::AddRange | ListChangeReason::RemoveRange | ListChangeReason::Moved => {}
            }
        }
    }

    create_change_set(result, all_groupings, initial_state_of_groups)
}

/// `Regroup`.
fn regroup<T: Clone + PartialEq + 'static, G: Eq + Hash + Clone + 'static>(
    state: &RefCell<GroupState<T, G>>,
    group_selector: &dyn Fn(&T) -> G,
    current_items: &[ItemWithGroupKey<T, G>],
) -> ChangeSet<ImmutableGroup<T, G>> {
    let mut state = state.borrow_mut();
    let GroupState { groupings: result, group_cache: all_groupings } = &mut *state;
    let mut initial_state_of_groups: Dictionary<G, ImmutableGroup<T, G>> = Dictionary::new();

    for item_with_value in current_items {
        let current_group_key = item_with_value.group();
        let new_group_key = group_selector(item_with_value.item());
        if new_group_key == current_group_key {
            continue;
        }

        // Find the old group, and remove the item from it.
        let old_grouping = get_group(all_groupings, &current_group_key);
        capture_initial_state(&mut initial_state_of_groups, &current_group_key, old_grouping);
        remove_item(old_grouping, item_with_value.item());

        // Mark the item with its new group.
        item_with_value.set_group(new_group_key.clone());

        // Add to the new group.
        let new_grouping = get_group(all_groupings, &new_group_key);
        capture_initial_state(&mut initial_state_of_groups, &new_group_key, new_grouping);
        new_grouping.push(item_with_value.item().clone());
    }

    create_change_set(result, all_groupings, initial_state_of_groups)
}
