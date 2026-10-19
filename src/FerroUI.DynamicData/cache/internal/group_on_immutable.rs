//! `GroupOnImmutable<TObject, TKey, TGroupKey>` (`Cache/Internal/GroupOnImmutable.cs`).

use super::group_on::{group_by, ChangeWithGroup, MissingKeyError};
use super::Cache;
use crate::cache::{Change, ChangeReason, ChangeSet, ICache, IGrouping, IQuery, ImmutableGroup, ImmutableGroupChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The groups of the items of `source` by `group_selector_key`, as states that do not change,
/// evaluated again for every item when `regrouper` produces a value (`GroupOnImmutable.Run`).
pub(crate) fn group_on_immutable<T, K, G>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    group_selector_key: Rc<dyn Fn(&T) -> G>,
    regrouper: Option<Rc<dyn IObservable<()>>>,
) -> Rc<dyn IObservable<ImmutableGroupChangeSet<T, K, G>>>
where
    T: Clone + 'static,
    K: Eq + Hash + Clone + 'static,
    G: Eq + Hash + Clone + 'static,
{
    let regrouper = regrouper.unwrap_or_else(Observable::never);
    Observable::create(move |observer: Rc<dyn IObserver<ImmutableGroupChangeSet<T, K, G>>>| {
        let grouper = Rc::new(RefCell::new(Grouper::new(group_selector_key.clone())));

        let updater = grouper.clone();
        let groups = source.try_select(move |updates: ChangeSet<T, K>| updater.borrow_mut().update(&updates));

        let regrouping = grouper.clone();
        let regroup = regrouper.try_select(move |_: ()| regrouping.borrow_mut().regroup());

        groups.merge(regroup).where_(|changes: &ImmutableGroupChangeSet<T, K, G>| changes.count() != 0).subscribe(observer)
    })
}

/// The items of a group (`GroupCache`).
type GroupCache<T, K> = Rc<RefCell<Cache<T, K>>>;

/// The state of a subscription to `GroupOnImmutable` (`Grouper`).
struct Grouper<T, K, G> {
    group_selector_key: Rc<dyn Fn(&T) -> G>,
    all_groupings: Dictionary<G, GroupCache<T, K>>,
    item_cache: Dictionary<K, ChangeWithGroup<T, K, G>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G: Eq + Hash + Clone + 'static> Grouper<T, K, G> {
    fn new(group_selector_key: Rc<dyn Fn(&T) -> G>) -> Self {
        Self { group_selector_key, all_groupings: Dictionary::new(), item_cache: Dictionary::new() }
    }

    /// Evaluates the group of every item again (`Regroup`).
    fn regroup(&mut self) -> Result<ImmutableGroupChangeSet<T, K, G>, ObservableError> {
        let items: Vec<Change<T, K>> = self.item_cache.iter().map(|(key, item)| Change::new(ChangeReason::Refresh, key.clone(), item.item.clone())).collect();
        self.handle_updates(&items)
    }

    /// Groups the changes of the source (`Update`).
    fn update(&mut self, updates: &ChangeSet<T, K>) -> Result<ImmutableGroupChangeSet<T, K, G>, ObservableError> {
        self.handle_updates(updates.changes())
    }

    fn missing_key_error(reason: ChangeReason) -> ObservableError {
        Rc::new(MissingKeyError(format!(
            "A key is missing from previous group on {reason:?}.\nObject type {}, Key type {}, Group key type {}",
            std::any::type_name::<T>(),
            std::any::type_name::<K>(),
            std::any::type_name::<G>()
        )))
    }

    /// The change set of the groups whose state changed, from their states before (`CreateChangeSet`).
    fn create_change_set(&mut self, initial_state_of_groups: Dictionary<G, ImmutableGroup<T, K, G>>) -> ImmutableGroupChangeSet<T, K, G> {
        let mut result = Vec::new();
        for (key, initial_group) in initial_state_of_groups.iter() {
            let current = self.all_groupings.get(key).cloned().expect("every group of the initial states is a grouping");
            if current.borrow().count() == 0 {
                self.all_groupings.remove(key);
                result.push(Change::new(ChangeReason::Remove, key.clone(), initial_group.clone()));
            } else {
                let current_state = ImmutableGroup::new(key.clone(), &*current.borrow());
                if initial_group.count() == 0 {
                    result.push(Change::new(ChangeReason::Add, key.clone(), current_state));
                } else {
                    result.push(Change::update(key.clone(), current_state, initial_group.clone()));
                }
            }
        }
        ImmutableGroupChangeSet::from(result)
    }

    /// The items of the group of `key`, made when there are none (`GetCache`).
    fn get_cache(&mut self, key: &G) -> GroupCache<T, K> {
        if let Some(cache) = self.all_groupings.get(key) {
            return cache.clone();
        }
        let new_cache = Rc::new(RefCell::new(Cache::new()));
        self.all_groupings.insert(key.clone(), new_cache.clone());
        new_cache
    }

    /// `HandleUpdates`.
    fn handle_updates(&mut self, changes: &[Change<T, K>]) -> Result<ImmutableGroupChangeSet<T, K, G>, ObservableError> {
        // The groups that change, with their states before, to make the right notifications.
        let mut initial_state_of_groups: Dictionary<G, ImmutableGroup<T, K, G>> = Dictionary::new();

        // 1. Group all items.
        let selector = self.group_selector_key.clone();
        let grouped = group_by(changes.iter().map(|change| ChangeWithGroup::new(change, &*selector)), |change| change.group_key.clone());

        // 2. Iterate and maintain the child caches.
        for (group_key, group) in grouped {
            let cache_to_modify = self.get_cache(&group_key);
            if !initial_state_of_groups.contains_key(&group_key) {
                let state = ImmutableGroup::new(group_key.clone(), &*cache_to_modify.borrow());
                initial_state_of_groups.insert(group_key.clone(), state);
            }

            // Maintain the group with its changes.
            for current in group {
                match current.reason {
                    ChangeReason::Add => {
                        cache_to_modify.borrow_mut().add_or_update(current.item.clone(), current.key.clone());
                        self.item_cache.insert(current.key.clone(), current);
                    }
                    ChangeReason::Update => {
                        cache_to_modify.borrow_mut().add_or_update(current.item.clone(), current.key.clone());

                        // Whether the previous item was in another group; if so, remove it from the
                        // old group.
                        let Some(previous_group_key) = self.item_cache.get(&current.key).map(|previous| previous.group_key.clone()) else {
                            return Err(Self::missing_key_error(ChangeReason::Update));
                        };
                        if previous_group_key != current.group_key {
                            self.remove_from_old_group(&mut initial_state_of_groups, &previous_group_key, &current.key);
                        }
                        self.item_cache.insert(current.key.clone(), current);
                    }
                    ChangeReason::Remove => {
                        let existing = cache_to_modify.borrow().lookup(&current.key);
                        if existing.is_some() {
                            cache_to_modify.borrow_mut().remove(&current.key);
                        } else {
                            // Removed because an evaluation of the group led to a removal.
                            let Some(previous_group_key) = self.item_cache.get(&current.key).map(|previous| previous.group_key.clone()) else {
                                return Err(Self::missing_key_error(ChangeReason::Remove));
                            };
                            self.remove_from_old_group(&mut initial_state_of_groups, &previous_group_key, &current.key);
                        }
                        self.item_cache.remove(&current.key);
                    }
                    ChangeReason::Refresh => {
                        // Whether the previous item was in another group; if so, remove it from the
                        // old group.
                        let previous_group_key = self.item_cache.get(&current.key).map(|previous| previous.group_key.clone());
                        match previous_group_key {
                            Some(previous_group_key) if previous_group_key == current.group_key => {}
                            Some(previous_group_key) => {
                                self.remove_from_old_group(&mut initial_state_of_groups, &previous_group_key, &current.key);
                                // Add to the new group, as the value of the group has changed.
                                cache_to_modify.borrow_mut().add_or_update(current.item.clone(), current.key.clone());
                            }
                            // It must be made, as it was added.
                            None => cache_to_modify.borrow_mut().add_or_update(current.item.clone(), current.key.clone()),
                        }
                        self.item_cache.insert(current.key.clone(), current);
                    }
                    ChangeReason::Moved => {}
                }
            }
        }

        // 2. Make the notifications, comparing the current and the previous states.
        Ok(self.create_change_set(initial_state_of_groups))
    }

    /// `RemoveFromOldGroup`.
    fn remove_from_old_group(&mut self, group_state: &mut Dictionary<G, ImmutableGroup<T, K, G>>, group_key: &G, current_key: &K) {
        let Some(group) = self.all_groupings.get(group_key).cloned() else { return };
        if !group_state.contains_key(group_key) {
            let state = ImmutableGroup::new(group_key.clone(), &*group.borrow());
            group_state.insert(group_key.clone(), state);
        }
        group.borrow_mut().remove(current_key);
    }
}
