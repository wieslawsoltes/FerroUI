//! `LeftJoin<TLeft, TLeftKey, TRight, TRightKey, TDestination>` (`Cache/Internal/LeftJoin.cs`).

use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache, IConnectableCache, IObservableCache, ObservableCacheEx};
use crate::internal::{synchronize_safe_shared, SharedDeliveryQueue};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

/// Each left item with the right item whose key `right_key_selector` maps to its key, if any,
/// keyed by the left key (`LeftJoin.Run`): a result exists while the left item does.
///
/// Both sources go through one [`SharedDeliveryQueue`], as upstream's; the lock that upstream holds
/// while it subscribes serializes nothing on one thread.
pub(crate) fn left_join<TL, KL, TR, KR, D>(
    left: Rc<dyn IObservable<ChangeSet<TL, KL>>>,
    right: Rc<dyn IObservable<ChangeSet<TR, KR>>>,
    right_key_selector: Rc<dyn Fn(&TR) -> KL>,
    result_selector: Rc<dyn Fn(&KL, &TL, Option<&TR>) -> D>,
) -> Rc<dyn IObservable<ChangeSet<D, KL>>>
where
    TL: Clone + 'static,
    KL: Eq + Hash + Clone + 'static,
    TR: Clone + 'static,
    KR: Eq + Hash + Clone + 'static,
    D: Clone + 'static,
{
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D, KL>>>| {
        let queue = SharedDeliveryQueue::new();

        // create local backing stores
        let left_share = synchronize_safe_shared(left.clone(), &queue).publish();
        let left_values: Rc<dyn IObservable<ChangeSet<TL, KL>>> = left_share.clone();
        let left_cache = left_values.as_observable_cache();

        let right_share = synchronize_safe_shared(right.clone(), &queue).publish();
        let right_values: Rc<dyn IObservable<ChangeSet<TR, KR>>> = right_share.clone();
        let right_cache = right_values.as_observable_cache();
        let right_foreign_cache = {
            let right_key_selector = right_key_selector.clone();
            right_values.change_key(move |item: &TR| right_key_selector(item)).as_observable_cache()
        };

        let right_foreign_keys_by_key: Rc<RefCell<HashMap<KR, KL>>> = Rc::new(RefCell::new(HashMap::new()));

        // joined is the final cache
        let joined = Rc::new(RefCell::new(ChangeAwareCache::<D, KL>::new()));

        let has_initialized = Rc::new(Cell::new(false));

        let left_loader = {
            let (joined, right_foreign_cache, result_selector) = (joined.clone(), right_foreign_cache.clone(), result_selector.clone());
            left_cache.connect().select(move |changes: ChangeSet<TL, KL>| {
                let mut joined = joined.borrow_mut();
                for change in &changes {
                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => {
                            // Update with left (and right if it is presents)
                            let left_current = change.current();
                            let right_lookup = right_foreign_cache.lookup(change.key());
                            let destination = result_selector(change.key(), left_current, right_lookup.as_ref());
                            joined.add_or_update(destination, change.key().clone());
                        }
                        // remove from result because a left value is expected
                        ChangeReason::Remove => joined.remove(change.key()),
                        // propagate upstream
                        ChangeReason::Refresh => joined.refresh(change.key()),
                        ChangeReason::Moved => {}
                    }
                }
                joined.capture_changes()
            })
        };

        let right_loader = {
            let (joined, left_cache, right_key_selector, result_selector, right_foreign_keys_by_key, has_initialized) = (
                joined.clone(),
                left_cache.clone(),
                right_key_selector.clone(),
                result_selector.clone(),
                right_foreign_keys_by_key.clone(),
                has_initialized.clone(),
            );
            right_cache
                .connect()
                .select(move |changes: ChangeSet<TR, KR>| {
                    let mut joined = joined.borrow_mut();
                    let mut right_foreign_keys_by_key = right_foreign_keys_by_key.borrow_mut();
                    for change in &changes {
                        let right = change.current();
                        let foreign_key = right_key_selector(change.current());
                        let left = left_cache.lookup(&foreign_key);

                        match change.reason() {
                            ChangeReason::Add | ChangeReason::Update => {
                                let prior_foreign_key = right_foreign_keys_by_key.get(change.key()).filter(|prior| **prior != foreign_key).cloned();
                                if let Some(prior_foreign_key) = prior_foreign_key {
                                    if let Some(prior_left) = left_cache.lookup(&prior_foreign_key) {
                                        let destination = result_selector(&prior_foreign_key, &prior_left, None);
                                        joined.add_or_update(destination, prior_foreign_key);
                                    }
                                }

                                if let Some(left) = &left {
                                    let destination = result_selector(&foreign_key, left, Some(right));
                                    joined.add_or_update(destination, foreign_key.clone());
                                }

                                right_foreign_keys_by_key.insert(change.key().clone(), foreign_key);
                            }
                            ChangeReason::Remove => {
                                if let Some(left) = &left {
                                    let destination = result_selector(&foreign_key, left, None);
                                    joined.add_or_update(destination, foreign_key);
                                }

                                right_foreign_keys_by_key.remove(change.key());
                            }
                            ChangeReason::Refresh => {
                                let prior_foreign_key = right_foreign_keys_by_key.get(change.key()).filter(|prior| **prior != foreign_key).cloned();
                                match prior_foreign_key {
                                    Some(prior_foreign_key) => {
                                        if let Some(prior_left) = left_cache.lookup(&prior_foreign_key) {
                                            let destination = result_selector(&prior_foreign_key, &prior_left, None);
                                            joined.add_or_update(destination, prior_foreign_key);
                                        }

                                        if let Some(left) = &left {
                                            let destination = result_selector(&foreign_key, left, Some(right));
                                            joined.add_or_update(destination, foreign_key.clone());
                                        }

                                        right_foreign_keys_by_key.insert(change.key().clone(), foreign_key);
                                    }
                                    None => joined.refresh(&foreign_key),
                                }
                            }
                            ChangeReason::Moved => {}
                        }
                    }
                    joined.capture_changes()
                })
                // Don't forward initial changesets from the right side, only the left
                .where_(move |_| has_initialized.get())
        };

        let observer_subscription = left_loader.merge(right_loader).subscribe_safe(observer);

        let right_share_connection = right_share.connect();

        has_initialized.set(true);

        let disposables: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([
            observer_subscription,
            Rc::new(left_cache) as Rc<dyn IDisposable>,
            Rc::new(right_cache) as Rc<dyn IDisposable>,
            right_share_connection,
            left_share.connect(),
            queue as Rc<dyn IDisposable>,
            // Deviation (DEVIATIONS.md, DynamicData): upstream does not dispose the cache of the foreign keys, which
            // stays subscribed to the shared right side; it is disposed here, last.
            Rc::new(right_foreign_cache) as Rc<dyn IDisposable>,
        ]);
        disposables
    })
}
