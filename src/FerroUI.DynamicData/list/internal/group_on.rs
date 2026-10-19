//! `GroupOn<TObject, TGroupKey>` (`List/Internal/GroupOn.cs`), with the parts the grouping
//! operators of lists share: the item with its group key (`ItemWithGroupKey`), the transform that
//! makes it (`Transform((t, previous) => ..., transformOnRefresh: true)`, after
//! `List/Internal/Transformer.cs`), and the items of the source as a collection (`ToCollection`).

use super::unified;
use crate::list::{clear_or_remove_many, clone_changes, remove_many, Change, ChangeAwareList, ChangeSet, Group, IExtendedList, IList, ListChangeReason};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

/// An index that the transform cannot find (`UnspecifiedIndexException`).
#[derive(Debug)]
pub(crate) struct UnspecifiedIndexError(pub(crate) String);

impl fmt::Display for UnspecifiedIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UnspecifiedIndexError {}

struct ItemInner<T, G> {
    item: T,
    group: RefCell<G>,
    previous_group: Option<G>,
}

/// An item with its group key, selected once, and the group key of the item it replaced
/// (`ItemWithGroupKey`). A regrouping changes the group key of the shared item, which every clone
/// sees; items are equal when their items are.
pub(crate) struct ItemWithGroupKey<T, G>(Rc<ItemInner<T, G>>);

impl<T, G> Clone for ItemWithGroupKey<T, G> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, G: Clone> ItemWithGroupKey<T, G> {
    fn new(item: T, group: G, previous_group: Option<G>) -> Self {
        Self(Rc::new(ItemInner { item, group: RefCell::new(group), previous_group }))
    }

    /// The item (`Item`).
    pub(crate) fn item(&self) -> &T {
        &self.0.item
    }

    /// The group key (`Group`).
    pub(crate) fn group(&self) -> G {
        self.0.group.borrow().clone()
    }

    /// Marks the item with its new group key (`Group { set; }`).
    pub(crate) fn set_group(&self, group: G) {
        *self.0.group.borrow_mut() = group;
    }

    /// The group key of the item this one replaced (`PreviousGroup`).
    pub(crate) fn previous_group(&self) -> Option<&G> {
        self.0.previous_group.as_ref()
    }
}

impl<T: PartialEq, G> PartialEq for ItemWithGroupKey<T, G> {
    /// Equal items (`Equals`, by `EqualityComparer<TObject>.Default`).
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.0.item == other.0.item
    }
}

/// An item of the source with what it was transformed into (`TransformedItemContainer`); equal
/// by the item of the source.
#[derive(Clone)]
struct TransformedItemContainer<T, G> {
    source: T,
    destination: ItemWithGroupKey<T, G>,
}

impl<T: PartialEq, G> PartialEq for TransformedItemContainer<T, G> {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

/// The items of `source` with their group keys, selected once; a refresh selects the group key
/// again, and keeps the previous one (`_source.Transform((t, previous) => new ItemWithGroupKey(t,
/// _groupSelector(t), previous.Convert(p => p.Group)), true)`).
pub(crate) fn items_with_group_key<T, G>(source: Rc<dyn IObservable<ChangeSet<T>>>, group_selector: Rc<dyn Fn(&T) -> G>) -> Rc<dyn IObservable<ChangeSet<ItemWithGroupKey<T, G>>>>
where
    T: Clone + PartialEq + 'static,
    G: Clone + 'static,
{
    Observable::defer(move || {
        let transformed = Rc::new(RefCell::new(ChangeAwareList::<TransformedItemContainer<T, G>>::new()));
        let group_selector = group_selector.clone();
        source.try_select(move |changes: ChangeSet<T>| {
            let mut transformed = transformed.borrow_mut();
            transform(&mut transformed, &changes, &*group_selector)?;
            let changed = transformed.capture_changes();
            Ok(map_changes(&changed, |container: &TransformedItemContainer<T, G>| container.destination.clone()))
        })
    })
}

/// The container of an item (`_containerFactory`).
fn container<T: Clone, G: Clone>(item: &T, previous: Option<&ItemWithGroupKey<T, G>>, group_selector: &dyn Fn(&T) -> G) -> TransformedItemContainer<T, G> {
    let destination = ItemWithGroupKey::new(item.clone(), group_selector(item), previous.map(ItemWithGroupKey::group));
    TransformedItemContainer { source: item.clone(), destination }
}

/// The index of the container of `item` (`FindItemIndex`).
fn find_item_index<T: Clone + PartialEq, G: Clone>(transformed: &ChangeAwareList<TransformedItemContainer<T, G>>, item: &T) -> Option<usize> {
    transformed.items().iter().position(|container| container.source == *item)
}

/// `Transformer.Transform`, with the transform on refresh.
fn transform<T: Clone + PartialEq, G: Clone>(
    transformed: &mut ChangeAwareList<TransformedItemContainer<T, G>>,
    changes: &ChangeSet<T>,
    group_selector: &dyn Fn(&T) -> G,
) -> Result<(), ObservableError> {
    for item in changes {
        match item.reason() {
            ListChangeReason::Add => {
                let change = item.item();
                match change.current_index() {
                    Some(index) if index < transformed.count() => {
                        let converted = container(change.current(), None, group_selector);
                        transformed.insert(index, converted);
                    }
                    _ => transformed.add(container(change.current(), None, group_selector)),
                }
            }
            ListChangeReason::AddRange => {
                let items: Vec<TransformedItemContainer<T, G>> = item.range().iter().map(|current| container(current, None, group_selector)).collect();
                IList::add_or_insert_range(&mut *transformed, items, item.range().index());
            }
            ListChangeReason::Refresh => {
                let change = item.item();
                let index = match change.current_index() {
                    Some(index) => index,
                    None => find_item_index(transformed, change.current()).ok_or_else(|| unspecified_index("Cannot find index of the refreshed item"))?,
                };
                let previous = transformed.get(index).destination.clone();
                let refreshed = container(change.current(), Some(&previous), group_selector);
                transformed.set(index, refreshed);
            }
            ListChangeReason::Replace => {
                let change = item.item();
                let previous_item = change.previous().expect("a replacement has a previous item");
                let previous_index = match change.previous_index() {
                    Some(index) => index,
                    None => find_item_index(transformed, previous_item).ok_or_else(|| unspecified_index("Cannot find index of the replaced item"))?,
                };
                let current_index = change.current_index().unwrap_or(previous_index);
                if current_index == previous_index {
                    let previous = transformed.get(previous_index).destination.clone();
                    let replacement = container(change.current(), Some(&previous), group_selector);
                    transformed.set(current_index, replacement);
                } else {
                    transformed.remove_at(previous_index);
                    transformed.insert(current_index, container(change.current(), None, group_selector));
                }
            }
            ListChangeReason::Remove => {
                let change = item.item();
                let index = match change.current_index() {
                    Some(index) => Some(index),
                    None => find_item_index(transformed, change.current()),
                };
                if let Some(index) = index {
                    transformed.remove_at(index);
                }
            }
            ListChangeReason::RemoveRange => match item.range().index() {
                Some(index) => transformed.remove_range(index, item.range().count()),
                None => {
                    let view: &ChangeAwareList<TransformedItemContainer<T, G>> = transformed;
                    let to_remove: Vec<TransformedItemContainer<T, G>> =
                        item.range().iter().filter_map(|current| find_item_index(view, current).map(|index| view.get(index).clone())).collect();
                    remove_many(&mut *transformed, &to_remove);
                }
            },
            ListChangeReason::Clear => {
                // The transformed items, to clear them right.
                let to_clear = Change::range_change(ListChangeReason::Clear, transformed.items().to_vec(), None);
                clear_or_remove_many(&mut *transformed, &to_clear);
            }
            ListChangeReason::Moved => {
                let change = item.item();
                let (Some(current_index), Some(previous_index)) = (change.current_index(), change.previous_index()) else {
                    return Err(unspecified_index("Cannot move as an index was not specified"));
                };
                IExtendedList::move_index(&mut *transformed, previous_index, current_index);
            }
        }
    }
    Ok(())
}

fn unspecified_index(message: &str) -> ObservableError {
    Rc::new(UnspecifiedIndexError(message.to_owned()))
}

/// The changes with their items mapped (`ChangeSetEx.Transform` of lists).
pub(crate) fn map_changes<S, D>(changes: &ChangeSet<S>, map: impl Fn(&S) -> D) -> ChangeSet<D> {
    changes
        .iter()
        .map(|change| match change.try_item() {
            Some(item) => Change::with_previous(change.reason(), map(item.current()), item.previous().map(&map), item.current_index(), item.previous_index()),
            None => Change::range_change(change.reason(), change.range().iter().map(&map).collect(), change.range().index()),
        })
        .collect()
}

/// The items of `source` after each change set (`ToCollection`, `QueryWhenChanged(items =>
/// items)`).
pub(crate) fn to_collection<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>) -> Rc<dyn IObservable<Rc<Vec<T>>>> {
    Observable::defer(move || {
        let list = Rc::new(RefCell::new(Vec::<T>::new()));
        source.select(move |changes: ChangeSet<T>| {
            let mut list = list.borrow_mut();
            clone_changes(&mut *list, &changes);
            Rc::new(list.clone())
        })
    })
}

/// LINQ's `GroupBy`: the groups in the order their keys first appear, each with its items in
/// order.
pub(crate) fn group_by<C, G: Eq + Hash + Clone>(items: impl IntoIterator<Item = C>, key: impl Fn(&C) -> G) -> Vec<(G, Vec<C>)> {
    let mut positions: HashMap<G, usize> = HashMap::new();
    let mut groups: Vec<(G, Vec<C>)> = Vec::new();
    for item in items {
        let group_key = key(&item);
        match positions.get(&group_key) {
            Some(&position) => groups[position].1.push(item),
            None => {
                positions.insert(group_key.clone(), groups.len());
                groups.push((group_key, vec![item]));
            }
        }
    }
    groups
}

/// Forwards the change sets of groups, then disposes the groups they remove or replace, and the
/// groups that are left when the sequence or the subscription ends (`DisposeMany` of lists).
fn dispose_many<T: Clone + PartialEq + 'static, G: PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<Group<T, G>>>>) -> Rc<dyn IObservable<ChangeSet<Group<T, G>>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Group<T, G>>>>| {
        let cached_items = Rc::new(RefCell::new(Vec::<Group<T, G>>::new()));
        let finalize = |cached_items: &RefCell<Vec<Group<T, G>>>| {
            let remaining = std::mem::take(&mut *cached_items.borrow_mut());
            for item in remaining {
                item.dispose();
            }
        };
        let (cached, on_error_cached, on_completed_cached) = (cached_items.clone(), cached_items.clone(), cached_items.clone());
        let (on_error, on_completed) = (observer.clone(), observer.clone());
        let subscription = source.subscribe_all(
            move |change_set: ChangeSet<Group<T, G>>| {
                observer.on_next(change_set.clone());
                for change in &change_set {
                    match change.reason() {
                        ListChangeReason::Clear => {
                            let items = cached.borrow().clone();
                            for item in items {
                                item.dispose();
                            }
                        }
                        ListChangeReason::Remove => change.item().current().dispose(),
                        ListChangeReason::RemoveRange => {
                            for item in change.range() {
                                item.dispose();
                            }
                        }
                        ListChangeReason::Replace => {
                            if let Some(previous) = change.item().previous() {
                                previous.dispose();
                            }
                        }
                        _ => {}
                    }
                }
                clone_changes(&mut *cached.borrow_mut(), &change_set);
            },
            move |error| {
                on_error.on_error(error);
                finalize(&on_error_cached);
            },
            move || {
                on_completed.on_completed();
                finalize(&on_completed_cached);
            },
        );
        Disposable::create(move || {
            subscription.dispose();
            finalize(&cached_items);
        })
    })
}

/// The state of a subscription to `GroupOn`: the groups as a list, and by group key.
struct GroupState<T, G> {
    groupings: ChangeAwareList<Group<T, G>>,
    group_cache: HashMap<G, Group<T, G>>,
}

/// The groups of the items of `source` by `group_selector`, evaluated again for every item when
/// `regrouper` produces a value (`GroupOn.Run`). The groups that are removed, and those left at the
/// end, are disposed, which completes their lists.
pub(crate) fn group_on<T, G>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    group_selector: Rc<dyn Fn(&T) -> G>,
    regrouper: Option<Rc<dyn IObservable<()>>>,
) -> Rc<dyn IObservable<ChangeSet<Group<T, G>>>>
where
    T: Clone + PartialEq + 'static,
    G: Eq + Hash + Clone + 'static,
{
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Group<T, G>>>>| {
        let state = Rc::new(RefCell::new(GroupState { groupings: ChangeAwareList::new(), group_cache: HashMap::new() }));

        // The group key of each item is selected once, up front.
        let items_with_group = items_with_group_key(source.clone(), group_selector.clone());

        // The `lock` of upstream serializes the source and the regrouper; on one thread they are
        // already serial.
        let shared = items_with_group.publish();
        let shared_items: Rc<dyn IObservable<ChangeSet<ItemWithGroupKey<T, G>>>> = shared.clone();

        let processing = state.clone();
        let grouper = shared_items.select(move |changes: ChangeSet<ItemWithGroupKey<T, G>>| process(&processing, &changes));

        let regrouper_func: Rc<dyn IObservable<ChangeSet<Group<T, G>>>> = match &regrouper {
            None => Observable::never(),
            Some(regrouper) => {
                let (regrouping, selector) = (state.clone(), group_selector.clone());
                regrouper.combine_latest(to_collection(shared_items.clone()), move |_: (), collection: Rc<Vec<ItemWithGroupKey<T, G>>>| {
                    regroup(&regrouping, &*selector, &collection)
                })
            }
        };

        // The groups are disposed when they are removed.
        let publisher = dispose_many(grouper.merge(regrouper_func)).where_(|changes: &ChangeSet<Group<T, G>>| changes.count() != 0).subscribe(observer);

        let connected = shared.connect();
        Disposable::create(move || {
            publisher.dispose();
            connected.dispose();
        })
    })
}

/// The group of `key`, made when there is none, and whether it was made (`GetCache`).
fn get_cache<T: Clone + PartialEq + 'static, G: Eq + Hash + Clone>(group_caches: &mut HashMap<G, Group<T, G>>, key: &G) -> (Group<T, G>, bool) {
    if let Some(cache) = group_caches.get(key) {
        return (cache.clone(), false);
    }
    let new_cache = Group::new(key.clone());
    group_caches.insert(key.clone(), new_cache.clone());
    (new_cache, true)
}

/// Removes `item` from the group of `group_key`, and the group when that empties it.
fn remove_from_old_group<T: Clone + PartialEq + 'static, G: Eq + Hash + Clone>(
    group_collection: &mut HashMap<G, Group<T, G>>,
    result: &mut ChangeAwareList<Group<T, G>>,
    group_key: &G,
    item: &T,
) {
    let Some(old_group) = group_collection.get(group_key).cloned() else { return };
    old_group.edit(|old_list| {
        old_list.remove(item);
    });
    if old_group.count() != 0 {
        return;
    }
    group_collection.remove(old_group.group_key());
    result.remove(&old_group);
}

/// `Process`.
fn process<T: Clone + PartialEq + 'static, G: Eq + Hash + Clone + 'static>(state: &RefCell<GroupState<T, G>>, changes: &ChangeSet<ItemWithGroupKey<T, G>>) -> ChangeSet<Group<T, G>> {
    let mut state = state.borrow_mut();
    let GroupState { groupings: result, group_cache: group_collection } = &mut *state;

    for (current_group, grouping) in group_by(unified(changes), |change| change.current.group()) {
        // Find the group, and add it to the result when it is made.
        let (group_cache, was_created) = get_cache(group_collection, &current_group);
        if was_created {
            result.add(group_cache.clone());
        }

        // One edit of the group, so that its changes come together.
        group_cache.edit(|list| {
            for change in grouping {
                match change.reason {
                    ListChangeReason::Add => list.add(change.current.item().clone()),
                    ListChangeReason::Replace => {
                        let previous = change.previous.as_ref().expect("a replacement has a previous item");
                        let previous_item = previous.item();
                        let previous_group = previous.group();

                        // Whether the change of the item moved it to another group.
                        if previous_group == current_group {
                            // Find and replace.
                            let index = list.index_of(previous_item).expect("the replaced item is in its group");
                            list.set(index, change.current.item().clone());
                        } else {
                            // Add to the new group, and remove from the old one.
                            list.add(change.current.item().clone());
                            remove_from_old_group(group_collection, result, &previous_group, previous_item);
                        }
                    }
                    ListChangeReason::Refresh => {
                        // 1. Whether the item was in the group and should not be now, or the
                        // other way.
                        let current_item = change.current.item().clone();
                        let previous_group = change.current.previous_group().cloned().expect("a refreshed item has the group key of the item it replaced");

                        if previous_group == current_group {
                            // Propagate the refresh.
                            if let Some(index) = list.index_of(&current_item) {
                                list.refresh_at(index, current_item);
                            }
                        } else {
                            // Add to the new group, and remove from the old one if that empties it.
                            list.add(current_item.clone());
                            remove_from_old_group(group_collection, result, &previous_group, &current_item);
                        }
                    }
                    ListChangeReason::Remove => {
                        list.remove(change.current.item());
                    }
                    ListChangeReason::Clear => list.clear(),
                    ListChangeReason::AddRange | ListChangeReason::RemoveRange | ListChangeReason::Moved => {}
                }
            }
        });

        if group_cache.count() == 0 {
            group_collection.remove(group_cache.group_key());
            result.remove(&group_cache);
        }
    }

    result.capture_changes()
}

/// `Regroup`.
fn regroup<T: Clone + PartialEq + 'static, G: Eq + Hash + Clone + 'static>(
    state: &RefCell<GroupState<T, G>>,
    group_selector: &dyn Fn(&T) -> G,
    current_items: &[ItemWithGroupKey<T, G>],
) -> ChangeSet<Group<T, G>> {
    let mut state = state.borrow_mut();
    let GroupState { groupings: result, group_cache: group_collection } = &mut *state;

    for item_with_value in current_items {
        let current_group_key = item_with_value.group();
        let new_group_key = group_selector(item_with_value.item());
        if new_group_key == current_group_key {
            continue;
        }

        // Remove from the old group.
        let (current_group_cache, _) = get_cache(group_collection, &current_group_key);
        current_group_cache.edit(|inner_list| {
            inner_list.remove(item_with_value.item());
        });
        if current_group_cache.count() == 0 {
            group_collection.remove(&current_group_key);
            result.remove(&current_group_cache);
        }

        // Mark the item with its new group.
        item_with_value.set_group(new_group_key.clone());

        // Add to the new group.
        let (new_group_cache, was_created) = get_cache(group_collection, &new_group_key);
        new_group_cache.edit(|inner_list| inner_list.add(item_with_value.item().clone()));
        if was_created {
            result.add(new_group_cache);
        }
    }

    result.capture_changes()
}
