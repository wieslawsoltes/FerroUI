//! `RightJoin<TLeft, TLeftKey, TRight, TRightKey, TDestination>` (`Cache/Internal/RightJoin.cs`).

use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache, IConnectableCache, IObservableCache, ObservableCacheEx};
use crate::internal::{synchronize_safe_shared, SharedDeliveryQueue};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

/// Each right item with the left item of the key `right_key_selector` maps it to, if any, keyed by
/// the right key (`RightJoin.Run`): a result exists while the right item does.
///
/// Both sources go through one [`SharedDeliveryQueue`], as upstream's; the lock that upstream holds
/// while it subscribes serializes nothing on one thread.
pub(crate) fn right_join<TL, KL, TR, KR, D>(
    left: Rc<dyn IObservable<ChangeSet<TL, KL>>>,
    right: Rc<dyn IObservable<ChangeSet<TR, KR>>>,
    right_key_selector: Rc<dyn Fn(&TR) -> KL>,
    result_selector: Rc<dyn Fn(&KR, Option<&TL>, &TR) -> D>,
) -> Rc<dyn IObservable<ChangeSet<D, KR>>>
where
    TL: Clone + 'static,
    KL: Eq + Hash + Clone + 'static,
    TR: Clone + 'static,
    KR: Eq + Hash + Clone + 'static,
    D: Clone + 'static,
{
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D, KR>>>| {
        let queue = SharedDeliveryQueue::new();

        // create local backing stores
        let left_cache = synchronize_safe_shared(left.clone(), &queue).as_observable_cache();

        let right_share = synchronize_safe_shared(right.clone(), &queue).publish();
        let right_values: Rc<dyn IObservable<ChangeSet<TR, KR>>> = right_share.clone();

        let right_cache = right_values.as_observable_cache();
        let right_foreign_cache = {
            let right_key_selector = right_key_selector.clone();
            right_values
                .transform_with_key(|item: &TR, key: &KR| (item.clone(), key.clone()))
                .change_key(move |pair: &(TR, KR)| right_key_selector(&pair.0))
                .as_observable_cache()
        };

        let right_foreign_keys_by_key: Rc<RefCell<HashMap<KR, KL>>> = Rc::new(RefCell::new(HashMap::new()));

        // joined is the final cache
        let joined_cache = Rc::new(RefCell::new(ChangeAwareCache::<D, KR>::new()));

        let has_initialized = Rc::new(Cell::new(false));

        let right_loader = {
            let (joined_cache, left_cache, right_key_selector, result_selector, right_foreign_keys_by_key) =
                (joined_cache.clone(), left_cache.clone(), right_key_selector.clone(), result_selector.clone(), right_foreign_keys_by_key.clone());
            right_cache.connect().select(move |changes: ChangeSet<TR, KR>| {
                let mut joined_cache = joined_cache.borrow_mut();
                let mut right_foreign_keys_by_key = right_foreign_keys_by_key.borrow_mut();
                for change in &changes {
                    let foreign_key = right_key_selector(change.current());
                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => {
                            // Update with right (and right if it is presents)
                            let right_current = change.current();
                            let left_lookup = left_cache.lookup(&foreign_key);
                            let destination = result_selector(change.key(), left_lookup.as_ref(), right_current);
                            joined_cache.add_or_update(destination, change.key().clone());

                            right_foreign_keys_by_key.insert(change.key().clone(), foreign_key);
                        }
                        ChangeReason::Remove => {
                            // remove from result because a right value is expected
                            joined_cache.remove(change.key());
                            right_foreign_keys_by_key.remove(change.key());
                        }
                        ChangeReason::Refresh => {
                            let has_new_foreign_key = right_foreign_keys_by_key.get(change.key()).is_some_and(|prior| *prior != foreign_key);
                            if has_new_foreign_key {
                                let left_lookup = left_cache.lookup(&foreign_key);
                                let destination = result_selector(change.key(), left_lookup.as_ref(), change.current());
                                joined_cache.add_or_update(destination, change.key().clone());

                                right_foreign_keys_by_key.insert(change.key().clone(), foreign_key);
                            } else {
                                // propagate downstream
                                joined_cache.refresh(change.key());
                            }
                        }
                        ChangeReason::Moved => {}
                    }
                }
                joined_cache.capture_changes()
            })
        };

        let left_loader = {
            let (joined_cache, right_foreign_cache, result_selector, has_initialized) =
                (joined_cache.clone(), right_foreign_cache.clone(), result_selector.clone(), has_initialized.clone());
            left_cache
                .connect()
                .select(move |changes: ChangeSet<TL, KL>| {
                    let mut joined_cache = joined_cache.borrow_mut();
                    for change in &changes {
                        let left = change.current();
                        if let Some((right_item, right_key)) = right_foreign_cache.lookup(change.key()) {
                            match change.reason() {
                                ChangeReason::Add | ChangeReason::Update => {
                                    let destination = result_selector(&right_key, Some(left), &right_item);
                                    joined_cache.add_or_update(destination, right_key);
                                }
                                ChangeReason::Remove => {
                                    let destination = result_selector(&right_key, None, &right_item);
                                    joined_cache.add_or_update(destination, right_key);
                                }
                                ChangeReason::Refresh => joined_cache.refresh(&right_key),
                                ChangeReason::Moved => {}
                            }
                        }
                    }
                    joined_cache.capture_changes()
                })
                // Don't forward initial changesets from the left side, only the right
                .where_(move |_| has_initialized.get())
        };

        let observer_subscription = left_loader.merge(right_loader).subscribe_safe(observer);

        has_initialized.set(true);

        let disposables: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([
            observer_subscription,
            Rc::new(left_cache) as Rc<dyn IDisposable>,
            Rc::new(right_cache) as Rc<dyn IDisposable>,
            right_share.connect(),
            queue as Rc<dyn IDisposable>,
            // Deviation (DEVIATIONS.md, DynamicData): upstream does not dispose the cache of the foreign keys, which
            // stays subscribed to the shared right side; it is disposed here, last.
            Rc::new(right_foreign_cache) as Rc<dyn IDisposable>,
        ]);
        disposables
    })
}
