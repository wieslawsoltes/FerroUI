//! `GroupOn<TObject, TKey, TGroupKey>` (`Cache/Internal/GroupOn.cs`), with the parts the grouping
//! operators of caches share: the change of an item with its group key (`ChangeWithGroup`), the
//! suspensions of the caches of groups (`SuspendTracker`), LINQ's `GroupBy` in order, and the
//! disposal of the groups that leave a sequence (`DisposeMany`).

use crate::cache::{Change, ChangeReason, ChangeSet, Group, GroupChangeSet, IChangeSet};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

/// An item that the grouping cannot find where it should be (`MissingKeyException`).
#[derive(Debug)]
pub(crate) struct MissingKeyError(pub(crate) String);

impl fmt::Display for MissingKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MissingKeyError {}

/// The change of an item with the group key of its item (`ChangeWithGroup`): the key is selected
/// once per change.
#[derive(Clone)]
pub(crate) struct ChangeWithGroup<T, K, G> {
    pub(crate) item: T,
    pub(crate) key: K,
    pub(crate) group_key: G,
    pub(crate) reason: ChangeReason,
}

impl<T: Clone, K: Clone, G> ChangeWithGroup<T, K, G> {
    pub(crate) fn new(change: &Change<T, K>, key_selector: &dyn Fn(&T) -> G) -> Self {
        Self { item: change.current().clone(), key: change.key().clone(), group_key: key_selector(change.current()), reason: change.reason() }
    }
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

/// The suspensions of the notifications of the caches of groups, one per group key
/// (`SuspendTracker`): the groups that may have observers deliver their changes once, when the
/// suspensions end.
pub(crate) struct SuspendTracker<G> {
    tracked_keys: HashSet<G>,
    disposables: Vec<Rc<dyn IDisposable>>,
}

impl<G: Eq + Hash + Clone> SuspendTracker<G> {
    pub(crate) fn new() -> Self {
        Self { tracked_keys: HashSet::new(), disposables: Vec::new() }
    }

    /// Suspends the notifications of `group`, once per group key (`Add`).
    pub(crate) fn add<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(&mut self, group: &Group<T, K, G>) {
        if self.tracked_keys.insert(group.key().clone()) {
            self.disposables.push(group.suspend_notifications());
        }
    }

    /// The suspensions so far, to end, and a clean slate (`Reset`): the caller ends them once it no
    /// longer edits the groups.
    pub(crate) fn reset(&mut self) -> Vec<Rc<dyn IDisposable>> {
        self.tracked_keys.clear();
        std::mem::take(&mut self.disposables)
    }

    /// Ends the suspensions, in the order they began (`Dispose`).
    pub(crate) fn dispose(mut self) {
        for disposable in self.reset() {
            disposable.dispose();
        }
    }
}

/// Forwards the change sets of `source`, then disposes the items that they remove or replace,
/// and the items that are left when the subscription ends (`DisposeMany`). `same` tells whether
/// two items are the same one, which a replacement by itself does not dispose.
pub(crate) fn dispose_many<V, G, C>(source: Rc<dyn IObservable<C>>, same: fn(&V, &V) -> bool) -> Rc<dyn IObservable<C>>
where
    V: IDisposable + Clone + 'static,
    G: Eq + Hash + Clone + 'static,
    C: IChangeSet<V, G>,
{
    Observable::create(move |observer: Rc<dyn IObserver<C>>| {
        let tracked = Rc::new(RefCell::new(Dictionary::<G, V>::new()));
        let (target, on_error, on_completed) = (tracked.clone(), observer.clone(), observer.clone());
        let subscription = source.subscribe_all(
            move |changes: C| {
                observer.on_next(changes.clone());
                for change in changes.change_set() {
                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => {
                            let previous = target.borrow_mut().insert(change.key().clone(), change.current().clone());
                            if let Some(previous) = previous {
                                if !same(&previous, change.current()) {
                                    previous.dispose();
                                }
                            }
                        }
                        ChangeReason::Remove => {
                            let removed = target.borrow_mut().remove(change.key());
                            if let Some(removed) = removed {
                                removed.dispose();
                            }
                        }
                        ChangeReason::Refresh | ChangeReason::Moved => {}
                    }
                }
            },
            move |error| on_error.on_error(error),
            move || on_completed.on_completed(),
        );
        Disposable::create(move || {
            subscription.dispose();
            let remaining: Vec<V> = tracked.borrow().values().cloned().collect();
            tracked.borrow_mut().clear();
            for item in remaining {
                item.dispose();
            }
        })
    })
}

/// The groups of the items of `source` by `group_selector_key`, evaluated again for every item when
/// `regrouper` produces a value (`GroupOn.Run`). The groups that are removed, and those left when
/// the subscription ends, are disposed, which completes their caches.
pub(crate) fn group_on<T, K, G>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    group_selector_key: Rc<dyn Fn(&T) -> G>,
    regrouper: Option<Rc<dyn IObservable<()>>>,
) -> Rc<dyn IObservable<GroupChangeSet<T, K, G>>>
where
    T: Clone + 'static,
    K: Eq + Hash + Clone + 'static,
    G: Eq + Hash + Clone + 'static,
{
    let regrouper = regrouper.unwrap_or_else(Observable::never);
    Observable::create(move |observer: Rc<dyn IObserver<GroupChangeSet<T, K, G>>>| {
        // The `SharedDeliveryQueue` of upstream serializes the source and the regrouper; on one
        // thread they are already serial.
        let grouper = Rc::new(RefCell::new(Grouper::new(group_selector_key.clone())));

        let completion = observer.clone();
        let updater = grouper.clone();
        let groups = source
            .finally(move || completion.on_completed())
            .try_select(move |updates: ChangeSet<T, K>| Grouper::run(&updater, |grouper| grouper.update(&updates)));

        let regrouping = grouper.clone();
        let regroup = regrouper.try_select(move |_: ()| Grouper::run(&regrouping, Grouper::regroup));

        let published = groups.merge(regroup).where_(|changes: &GroupChangeSet<T, K, G>| changes.count() != 0).publish();
        let shared: Rc<dyn IObservable<GroupChangeSet<T, K, G>>> = published.clone();
        let subscriber = shared.subscribe(observer);
        // Deviation (DEVIATIONS.md, DynamicData): upstream subscribes the disposal of the groups
        // without an error handler (`Subscribe()`), which rethrows an error; here the error reaches
        // the observer only, through `subscriber`.
        let disposer = dispose_many(shared, Group::ptr_eq).subscribe_all(|_| {}, |_| {}, || {});

        let connected = published.connect();
        Disposable::create(move || {
            connected.dispose();
            disposer.dispose();
            subscriber.dispose();
        })
    })
}

/// The state of a subscription to `GroupOn` (`Grouper`).
struct Grouper<T, K, G> {
    group_selector_key: Rc<dyn Fn(&T) -> G>,
    group_cache: Dictionary<G, Group<T, K, G>>,
    item_cache: Dictionary<K, ChangeWithGroup<T, K, G>>,
}

/// What the grouping of one change set produces: its change set of groups, or the error of an
/// item that was not found, and the suspensions to end once the grouper is no longer borrowed.
type Outcome<T, K, G> = (Result<GroupChangeSet<T, K, G>, ObservableError>, SuspendTracker<G>);

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G: Eq + Hash + Clone + 'static> Grouper<T, K, G> {
    fn new(group_selector_key: Rc<dyn Fn(&T) -> G>) -> Self {
        Self { group_selector_key, group_cache: Dictionary::new(), item_cache: Dictionary::new() }
    }

    /// Runs `action` on the grouper, then ends the suspensions of the groups it edited (the
    /// `using` of the suspend tracker), so that the observers of the groups run while the grouper
    /// is free.
    fn run(grouper: &RefCell<Self>, action: impl FnOnce(&mut Self) -> Outcome<T, K, G>) -> Result<GroupChangeSet<T, K, G>, ObservableError> {
        let (result, suspensions) = action(&mut grouper.borrow_mut());
        suspensions.dispose();
        result
    }

    /// Evaluates the group of every item again (`Regroup`).
    fn regroup(&mut self) -> Outcome<T, K, G> {
        let items: Vec<Change<T, K>> = self.item_cache.iter().map(|(key, item)| Change::new(ChangeReason::Refresh, key.clone(), item.item.clone())).collect();
        self.handle_updates(&items, true)
    }

    /// Groups the changes of the source (`Update`).
    fn update(&mut self, updates: &ChangeSet<T, K>) -> Outcome<T, K, G> {
        self.handle_updates(updates.changes(), false)
    }

    /// The group of `key`, made when there is none, and whether it was made (`GetCache`).
    fn get_cache(&mut self, key: &G) -> (Group<T, K, G>, bool) {
        if let Some(cache) = self.group_cache.get(key) {
            return (cache.clone(), false);
        }
        let new_cache = Group::new(key.clone());
        self.group_cache.insert(key.clone(), new_cache.clone());
        (new_cache, true)
    }

    /// `HandleUpdates`: maintains the caches of the groups and the group of each item.
    fn handle_updates(&mut self, changes: &[Change<T, K>], is_regrouping: bool) -> Outcome<T, K, G> {
        let mut result: Vec<Change<Group<T, K, G>, G>> = Vec::new();
        let selector = self.group_selector_key.clone();
        // Group all items.
        let grouped = group_by(changes.iter().map(|change| ChangeWithGroup::new(change, &*selector)), |change| change.group_key.clone());
        let mut suspend_tracker = SuspendTracker::new();
        let mut error: Option<ObservableError> = None;

        // 1. Iterate and maintain the child caches (`_groupCache`).
        // 2. Maintain the group each item belongs to (`_itemCache`).
        for (group_key, group) in grouped {
            let (group_cache, was_created) = self.get_cache(&group_key);
            if was_created {
                result.push(Change::new(ChangeReason::Add, group_key.clone(), group_cache.clone()));
            } else {
                // It was not made now, so it may have observers: suspend its updates until the end.
                suspend_tracker.add(&group_cache);
            }

            let groups = &mut self.group_cache;
            let items = &mut self.item_cache;
            let removed = &mut result;
            let tracker = &mut suspend_tracker;
            let failure = &mut error;
            group_cache.update(|group_updater| {
                for current in group {
                    match current.reason {
                        ChangeReason::Add | ChangeReason::Update => {
                            group_updater.add_or_update_with_key(current.item.clone(), current.key.clone());

                            // Whether the previous item was in another group; if so, remove it from
                            // the old group.
                            let previous_group_key = items.get(&current.key).map(|previous| previous.group_key.clone());
                            if let Some(previous_group_key) = previous_group_key {
                                if previous_group_key != current.group_key {
                                    remove_from_group(groups, tracker, removed, &previous_group_key, &current.key);
                                }
                            }
                            items.insert(current.key.clone(), current);
                        }
                        ChangeReason::Remove => {
                            if group_updater.lookup(&current.key).is_some() {
                                group_updater.remove_key(&current.key);
                            } else {
                                // Removed because an evaluation of the group led to a removal.
                                let Some(previous_group_key) = items.get(&current.key).map(|previous| previous.group_key.clone()) else {
                                    *failure = Some(Rc::new(MissingKeyError(format!(
                                        "A key is missing from previous value on remove. Object type {}, Key type {}, Group key type {}",
                                        std::any::type_name::<T>(),
                                        std::any::type_name::<K>(),
                                        std::any::type_name::<G>()
                                    ))));
                                    return;
                                };
                                remove_from_group(groups, tracker, removed, &previous_group_key, &current.key);
                            }
                            // Finally, remove the item from the item cache.
                            items.remove(&current.key);
                        }
                        ChangeReason::Refresh => {
                            // Whether the previous item was in another group; if so, remove it from
                            // the old group.
                            let previous_group_key = items.get(&current.key).map(|previous| previous.group_key.clone());
                            match previous_group_key {
                                Some(previous_group_key) if previous_group_key == current.group_key => {
                                    // Propagate the evaluation up the chain.
                                    if !is_regrouping {
                                        group_updater.refresh_key(&current.key);
                                    }
                                }
                                Some(previous_group_key) => {
                                    remove_from_group(groups, tracker, removed, &previous_group_key, &current.key);
                                    group_updater.add_or_update_with_key(current.item.clone(), current.key.clone());
                                }
                                None => group_updater.add_or_update_with_key(current.item.clone(), current.key.clone()),
                            }
                            items.insert(current.key.clone(), current);
                        }
                        ChangeReason::Moved => {}
                    }
                }
            });
            if let Some(error) = error {
                return (Err(error), suspend_tracker);
            }

            if group_cache.count() != 0 {
                continue;
            }
            self.group_cache.remove(&group_key);
            result.push(Change::new(ChangeReason::Remove, group_key, group_cache));
        }

        (Ok(GroupChangeSet::from(result)), suspend_tracker)
    }
}

/// Removes the item of `key` from the group of `group_key`, and the group when that empties it
/// (the lambda of `_groupCache.Lookup(..).IfHasValue(..)`).
fn remove_from_group<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G: Eq + Hash + Clone + 'static>(
    groups: &mut Dictionary<G, Group<T, K, G>>,
    suspend_tracker: &mut SuspendTracker<G>,
    result: &mut Vec<Change<Group<T, K, G>, G>>,
    group_key: &G,
    key: &K,
) {
    let Some(group) = groups.get(group_key).cloned() else { return };
    suspend_tracker.add(&group);
    group.update(|updater| updater.remove_key(key));
    if group.count() != 0 {
        return;
    }
    groups.remove(group.key());
    result.push(Change::new(ChangeReason::Remove, group.key().clone(), group));
}
