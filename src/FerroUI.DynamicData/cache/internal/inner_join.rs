//! `InnerJoin<TLeft, TLeftKey, TRight, TRightKey, TDestination>` (`Cache/Internal/InnerJoin.cs`).

use super::group_on_immutable;
use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache, IConnectableCache, IGrouping, IObservableCache, ImmutableGroup, ObservableCache, ObservableCacheEx};
use crate::internal::{synchronize_safe_shared, SharedDeliveryQueue};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

/// The pairs of a left item and a right item whose key `right_key_selector` maps to the key of the
/// left item, keyed by both keys (`InnerJoin.Run`): a pair exists while both items do.
///
/// Both sources go through one [`SharedDeliveryQueue`], as upstream's; the lock that upstream holds
/// while it subscribes serializes nothing on one thread.
pub(crate) fn inner_join<TL, KL, TR, KR, D>(
    left: Rc<dyn IObservable<ChangeSet<TL, KL>>>,
    right: Rc<dyn IObservable<ChangeSet<TR, KR>>>,
    right_key_selector: Rc<dyn Fn(&TR) -> KL>,
    result_selector: Rc<dyn Fn(&(KL, KR), &TL, &TR) -> D>,
) -> Rc<dyn IObservable<ChangeSet<D, (KL, KR)>>>
where
    TL: Clone + 'static,
    KL: Eq + Hash + Clone + 'static,
    TR: Clone + 'static,
    KR: Eq + Hash + Clone + 'static,
    D: Clone + 'static,
{
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D, (KL, KR)>>>| {
        let queue = SharedDeliveryQueue::new();

        // create local backing stores
        let left_cache = synchronize_safe_shared(left.clone(), &queue).as_observable_cache();

        let right_share = synchronize_safe_shared(right.clone(), &queue).publish();
        let right_values: Rc<dyn IObservable<ChangeSet<TR, KR>>> = right_share.clone();
        let right_cache = right_values.as_observable_cache();
        let right_grouped = ObservableCache::<ImmutableGroup<TR, KR, KL>, KL>::from_source(group_on_immutable(right_values, right_key_selector.clone(), None));

        let right_foreign_keys_by_key: Rc<RefCell<HashMap<KR, KL>>> = Rc::new(RefCell::new(HashMap::new()));

        // joined is the final cache
        let joined_cache = Rc::new(RefCell::new(ChangeAwareCache::<D, (KL, KR)>::new()));

        let has_initialized = Rc::new(Cell::new(false));

        let left_loader = {
            let (joined_cache, right_grouped, result_selector, has_initialized) =
                (joined_cache.clone(), right_grouped.clone(), result_selector.clone(), has_initialized.clone());
            left_cache
                .connect()
                .select(move |changes: ChangeSet<TL, KL>| {
                    let mut joined_cache = joined_cache.borrow_mut();
                    for change in &changes {
                        let left_current = change.current();
                        if let Some(right_lookup) = right_grouped.lookup(change.key()) {
                            match change.reason() {
                                ChangeReason::Add | ChangeReason::Update => {
                                    for (right_key, right) in right_lookup.key_values() {
                                        let key = (change.key().clone(), right_key);
                                        let destination = result_selector(&key, left_current, &right);
                                        joined_cache.add_or_update(destination, key);
                                    }
                                }
                                ChangeReason::Remove => {
                                    for (right_key, _) in right_lookup.key_values() {
                                        joined_cache.remove(&(change.key().clone(), right_key));
                                    }
                                }
                                ChangeReason::Refresh => {
                                    for right_key in right_lookup.keys() {
                                        joined_cache.refresh(&(change.key().clone(), right_key));
                                    }
                                }
                                ChangeReason::Moved => {}
                            }
                        }
                    }
                    joined_cache.capture_changes()
                })
                // Don't forward initial changesets from the left side, only the right
                .where_(move |_| has_initialized.get())
        };

        let right_loader = {
            let (joined_cache, left_cache, right_key_selector, result_selector, right_foreign_keys_by_key) =
                (joined_cache.clone(), left_cache.clone(), right_key_selector.clone(), result_selector.clone(), right_foreign_keys_by_key.clone());
            right_cache.connect().select(move |changes: ChangeSet<TR, KR>| {
                let mut joined_cache = joined_cache.borrow_mut();
                let mut right_foreign_keys_by_key = right_foreign_keys_by_key.borrow_mut();
                for change in &changes {
                    let left_key = right_key_selector(change.current());
                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => {
                            // If this is an update and the foreign key has changed, we need to remove the old pairing before attempting to add a new one.
                            if let Some(old_left_key) = right_foreign_keys_by_key.get(change.key()) {
                                if left_key != *old_left_key && left_cache.lookup(old_left_key).is_some() {
                                    joined_cache.remove(&(old_left_key.clone(), change.key().clone()));
                                }
                            }

                            // If the new item has a pairing, either add or update it
                            right_foreign_keys_by_key.insert(change.key().clone(), left_key.clone());
                            let right = change.current();
                            if let Some(left) = left_cache.lookup(&left_key) {
                                let key = (left_key, change.key().clone());
                                let destination = result_selector(&key, &left, right);
                                joined_cache.add_or_update(destination, key);
                            }
                        }
                        ChangeReason::Remove => {
                            // remove from result because a right value is expected
                            right_foreign_keys_by_key.remove(change.key());
                            joined_cache.remove(&(left_key, change.key().clone()));
                        }
                        ChangeReason::Refresh => {
                            // Check to see if the foreign key has changed, and re-pair the item, if so.
                            // Deviation (DEVIATIONS.md, DynamicData): upstream reads the prior key with
                            // the indexer, which throws for a key never added; here such a refresh
                            // (of an invalid change set) is forwarded as a refresh.
                            let old_left_key = right_foreign_keys_by_key.insert(change.key().clone(), left_key.clone());
                            match old_left_key {
                                Some(old_left_key) if old_left_key != left_key => {
                                    if left_cache.lookup(&old_left_key).is_some() {
                                        joined_cache.remove(&(old_left_key, change.key().clone()));
                                    }

                                    if let Some(left) = left_cache.lookup(&left_key) {
                                        let key = (left_key, change.key().clone());
                                        let destination = result_selector(&key, &left, change.current());
                                        joined_cache.add_or_update(destination, key);
                                    }
                                }
                                _ => joined_cache.refresh(&(left_key, change.key().clone())),
                            }
                        }
                        ChangeReason::Moved => {}
                    }
                }
                joined_cache.capture_changes()
            })
        };

        let observer_subscription = left_loader.merge(right_loader).subscribe_safe(observer);

        has_initialized.set(true);

        let disposables: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([
            observer_subscription,
            Rc::new(left_cache) as Rc<dyn IDisposable>,
            Rc::new(right_cache) as Rc<dyn IDisposable>,
            right_share.connect(),
            queue as Rc<dyn IDisposable>,
            // Deviation (DEVIATIONS.md, DynamicData): upstream does not dispose the grouped cache, which
            // stays subscribed to the shared right side; it is disposed here, last.
            Rc::new(right_grouped) as Rc<dyn IDisposable>,
        ]);
        disposables
    })
}
