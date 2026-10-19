//! `DynamicGrouper<TObject, TKey, TGroupKey>` (`Cache/Internal/DynamicGrouper.cs`): the groups of
//! `GroupOnDynamic` and `GroupOnObservable`, whose group keys change.

use super::group_on::SuspendTracker;
use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, Group, GroupChangeSet, ICache, IObservableCache, IQuery};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObserver};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

/// The group key of an item and its key (`Func<TObject, TKey, TGroupKey>`).
pub(crate) type DynamicGroupSelector<T, K, G> = Rc<dyn Fn(&T, &K) -> G>;

/// `Debug.Fail`: a state that the grouping never reaches; it panics in a debug build only.
fn debug_fail(message: &str) {
    if cfg!(debug_assertions) {
        panic!("{message}");
    }
}

/// What the grouper has to do once it is no longer borrowed (the end of `EmitChanges`): dispose
/// the groups that became empty, end the suspensions of the groups, then deliver the change set of
/// the groups. The observers of the groups and of the result may edit the sources again, so they
/// run while the grouper is free.
pub(crate) struct Emission<T, K, G> {
    disposed_groups: Vec<Group<T, K, G>>,
    suspensions: Vec<Rc<dyn IDisposable>>,
    changes: Option<GroupChangeSet<T, K, G>>,
}

impl<T, K, G> Default for Emission<T, K, G> {
    fn default() -> Self {
        Self { disposed_groups: Vec::new(), suspensions: Vec::new(), changes: None }
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G: 'static> Emission<T, K, G> {
    /// Delivers the emission to `observer`.
    pub(crate) fn deliver(self, observer: &dyn IObserver<GroupChangeSet<T, K, G>>) {
        for group in self.disposed_groups {
            group.dispose();
        }
        for suspension in self.suspensions {
            suspension.dispose();
        }
        if let Some(changes) = self.changes {
            observer.on_next(changes);
        }
    }
}

/// The groups of items whose group keys are given or selected (`DynamicGrouper`).
pub(crate) struct DynamicGrouper<T, K, G> {
    group_cache: ChangeAwareCache<Group<T, K, G>, G>,
    group_keys: HashMap<K, G>,
    /// The groups that became empty (`_emptyGroups`, a set of groups by group key, enumerated in
    /// .NET's order).
    empty_groups: Dictionary<G, Group<T, K, G>>,
    suspend_tracker: SuspendTracker<G>,
    group_selector: Option<DynamicGroupSelector<T, K, G>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G: Eq + Hash + Clone + 'static> DynamicGrouper<T, K, G> {
    /// A grouper with a selector of group keys, or one whose group keys are given
    /// (`DynamicGrouper(Func<TObject, TKey, TGroupKey>? groupSelector = null)`).
    pub(crate) fn new(group_selector: Option<DynamicGroupSelector<T, K, G>>) -> Self {
        Self { group_cache: ChangeAwareCache::new(), group_keys: HashMap::new(), empty_groups: Dictionary::new(), suspend_tracker: SuspendTracker::new(), group_selector }
    }

    /// Puts the item of `key` in the group of `group_key` (`AddOrUpdate`); with `emit`, the changes
    /// are emitted, which needs no suspension as each group changes once.
    pub(crate) fn add_or_update(&mut self, key: K, group_key: G, item: T, emit: bool) -> Option<Emission<T, K, G>> {
        // If the changes are not emitted, suspend the notifications.
        self.perform_add_or_update(key, group_key, item, !emit);
        emit.then(|| self.emit_changes())
    }

    /// Groups the changes of `change_set` (`ProcessChangeSet`); with `emit`, the changes are
    /// emitted.
    pub(crate) fn process_change_set(&mut self, change_set: &ChangeSet<T, K>, emit: bool) -> Option<Emission<T, K, G>> {
        // An emitted change set of one change makes at most one change per group downstream, so
        // there is no value in suspending the notifications.
        let suspend = !(emit && change_set.count() == 1);
        for change in change_set {
            self.process_change_with(change, suspend);
        }
        emit.then(|| self.emit_changes())
    }

    /// Groups one change, suspending the groups it changes (`ProcessChange`).
    pub(crate) fn process_change(&mut self, change: &Change<T, K>) {
        self.process_change_with(change, true);
    }

    fn process_change_with(&mut self, change: &Change<T, K>, suspend: bool) {
        let selector = self.group_selector.clone();
        match (change.reason(), selector) {
            (ChangeReason::Add, Some(selector)) | (ChangeReason::Update, Some(selector)) => {
                let group_key = selector(change.current(), change.key());
                self.perform_add_or_update(change.key().clone(), group_key, change.current().clone(), suspend);
            }
            (ChangeReason::Remove, _) => self.perform_remove(change.key(), suspend),
            (ChangeReason::Update, None) => self.perform_update(change.key(), suspend),
            (ChangeReason::Refresh, Some(selector)) => {
                let group_key = selector(change.current(), change.key());
                self.perform_refresh_to(change.key().clone(), group_key, change.current().clone(), suspend);
            }
            (ChangeReason::Refresh, None) => self.perform_refresh(change.key(), suspend),
            _ => {}
        }
    }

    /// Evaluates the group of every item again and moves the items whose group changed, so that
    /// each group emits one change set (`RegroupAll`).
    pub(crate) fn regroup_all(&mut self) -> Emission<T, K, G> {
        let Some(selector) = self.group_selector.clone() else {
            debug_fail("RegroupAll called without a GroupSelector. No changes will be made.");
            return Emission::default();
        };

        // The items whose group keys changed.
        let mut group_changes = Vec::new();
        for group in self.group_cache.items() {
            for (key, item) in group.observable_cache().key_values().iter() {
                let new_group_key = selector(item, key);
                if *group.key() != new_group_key {
                    group_changes.push((key.clone(), item.clone(), group.clone(), new_group_key));
                }
            }
        }

        for (key, item, old_group, new_group_key) in group_changes {
            self.perform_group_add_or_update(key.clone(), new_group_key, item, true);
            self.suspend_tracker.add(&old_group);
            let mut became_empty = false;
            old_group.update(|updater| {
                updater.remove_key(&key);
                became_empty = updater.count() == 0;
            });
            if became_empty {
                self.add_empty_group(old_group);
            }
        }

        self.emit_changes()
    }

    /// Sets the selector of group keys and evaluates every item again (`SetGroupSelector`).
    pub(crate) fn set_group_selector(&mut self, group_selector: DynamicGroupSelector<T, K, G>) -> Emission<T, K, G> {
        self.group_selector = Some(group_selector);
        self.regroup_all()
    }

    /// Groups the items seen before the first selector of group keys (`Initialize`).
    pub(crate) fn initialize(&mut self, initial_values: Vec<(K, T)>, group_selector: DynamicGroupSelector<T, K, G>) -> Emission<T, K, G> {
        if self.group_selector.is_some() {
            debug_fail("Initialize called when a GroupSelector is already present. No changes will be made.");
            return Emission::default();
        }

        // No suspension: the caches of the groups cannot have observers yet.
        self.group_selector = Some(group_selector.clone());
        for (key, item) in initial_values {
            let group_key = group_selector(&item, &key);
            self.perform_add_or_update(key, group_key, item, false);
        }

        self.emit_changes()
    }

    /// Removes the groups that became empty and captures the changes of the groups (`EmitChanges`);
    /// the [`Emission`] disposes the empty groups, ends the suspensions and emits the changes.
    pub(crate) fn emit_changes(&mut self) -> Emission<T, K, G> {
        // Verify that no group that is not empty was captured.
        debug_assert!(self.empty_groups.values().all(|group| group.count() == 0), "Non empty Group in Empty Group HashSet");

        // Remove the empty groups, to dispose.
        let empty_groups: Vec<Group<T, K, G>> = self.empty_groups.values().cloned().collect();
        let mut disposed_groups = Vec::new();
        for group in empty_groups {
            if group.count() == 0 {
                self.group_cache.remove(group.key());
                disposed_groups.push(group);
            }
        }
        self.empty_groups.clear();

        // Make sure that no empty group was missed.
        debug_assert!(!self.group_cache.items().iter().any(|group| group.count() == 0), "Not all empty Groups were removed");

        let suspensions = self.suspend_tracker.reset();

        // The pending changes.
        let change_set = self.group_cache.capture_changes();
        let changes = (change_set.count() != 0).then(|| GroupChangeSet::from(change_set));
        Emission { disposed_groups, suspensions, changes }
    }

    /// What `Dispose` ends, for the caller to dispose once the grouper is no longer borrowed: the
    /// suspensions, then the groups.
    pub(crate) fn take_for_dispose(&mut self) -> (Vec<Rc<dyn IDisposable>>, Vec<Group<T, K, G>>) {
        (self.suspend_tracker.reset(), self.group_cache.items())
    }

    fn add_empty_group(&mut self, group: Group<T, K, G>) {
        if !self.empty_groups.contains_key(group.key()) {
            self.empty_groups.insert(group.key().clone(), group);
        }
    }

    fn perform_group_refresh(&mut self, key: &K, group: Option<Group<T, K, G>>, suspend: bool) {
        match group {
            Some(group) => {
                if suspend {
                    self.suspend_tracker.add(&group);
                }
                group.update(|updater| updater.refresh_key(key));
            }
            None => debug_fail("Should not receive a refresh for an unknown Group Key"),
        }
    }

    fn lookup_group_of(&self, key: &K) -> Option<Group<T, K, G>> {
        self.group_keys.get(key).and_then(|group_key| self.lookup_group(group_key))
    }

    fn lookup_group(&self, group_key: &G) -> Option<Group<T, K, G>> {
        self.group_cache.lookup(group_key)
    }

    fn get_or_add_group(&mut self, group_key: &G) -> Group<T, K, G> {
        if let Some(group) = self.lookup_group(group_key) {
            return group;
        }
        let new_group = Group::new(group_key.clone());
        self.group_cache.add(new_group.clone(), group_key.clone());
        new_group
    }

    fn perform_add_or_update(&mut self, key: K, group_key: G, item: T, suspend: bool) {
        // Whether the item has been grouped already.
        if let Some(current_group_key) = self.group_keys.get(&key).cloned() {
            if group_key == current_group_key {
                // The group key did not change: update the item in its group.
                if let Some(group) = self.lookup_group(&current_group_key) {
                    if suspend {
                        self.suspend_tracker.add(&group);
                    }
                    group.update(|updater| updater.add_or_update_with_key(item, key));
                    return;
                }
                debug_fail("If there is a GroupKey associated with a Key, the Group for that GroupKey should exist.");
            } else {
                // The group key changed: remove the item from its old group, and add it below.
                self.perform_remove_from(&key, &current_group_key, suspend);
            }
        }

        // Find the right group and add the item.
        self.perform_group_add_or_update(key, group_key, item, suspend);
    }

    fn perform_group_add_or_update(&mut self, key: K, group_key: G, item: T, suspend: bool) {
        let group = self.get_or_add_group(&group_key);
        if suspend {
            self.suspend_tracker.add(&group);
        }
        let item_key = key.clone();
        group.update(|updater| updater.add_or_update_with_key(item, item_key));
        self.group_keys.insert(key, group_key.clone());

        // It cannot be empty, as an item was just added.
        self.empty_groups.remove(&group_key);
    }

    fn perform_refresh(&mut self, key: &K, suspend: bool) {
        let group = self.lookup_group_of(key);
        self.perform_group_refresh(key, group, suspend);
    }

    /// A refresh with the new group key: the item moves when its group changed.
    fn perform_refresh_to(&mut self, key: K, new_group_key: G, item: T, suspend: bool) {
        let Some(group_key) = self.group_keys.get(&key).cloned() else {
            debug_fail("Should not receive a refresh for an unknown key");
            return;
        };
        if new_group_key == group_key {
            // The group key did not change: refresh the item in its group.
            let group = self.lookup_group(&group_key);
            self.perform_group_refresh(&key, group, suspend);
        } else {
            // The group key changed: remove the item from the old group and add it to the new.
            self.perform_remove_from(&key, &group_key, suspend);
            self.perform_group_add_or_update(key, new_group_key, item, suspend);
        }
    }

    fn perform_remove(&mut self, key: &K, suspend: bool) {
        let Some(group_key) = self.group_keys.get(key).cloned() else {
            debug_fail("Should not receive a Remove Event for an unknown key");
            return;
        };
        self.perform_remove_from(key, &group_key, suspend);
        self.group_keys.remove(key);
    }

    fn perform_remove_from(&mut self, key: &K, group_key: &G, suspend: bool) {
        let Some(current_group) = self.lookup_group(group_key) else {
            debug_fail("Should not receive a Remove Event for an unknown Group Key");
            return;
        };
        if suspend {
            self.suspend_tracker.add(&current_group);
        }
        let mut became_empty = false;
        current_group.update(|updater| {
            updater.remove_key(key);
            became_empty = updater.count() == 0;
        });
        if became_empty {
            self.add_empty_group(current_group);
        }
    }

    /// Without the new group key, the old item can only be removed; the caller adds the new one
    /// (`PerformUpdate`).
    fn perform_update(&mut self, key: &K, suspend: bool) {
        self.perform_remove(key, suspend);
    }
}
