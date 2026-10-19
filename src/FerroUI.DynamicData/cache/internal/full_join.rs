//! `FullJoin<TLeft, TLeftKey, TRight, TRightKey, TDestination>` (`Cache/Internal/FullJoin.cs`).

use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache, IConnectableCache, IObservableCache, ObservableCacheEx};
use crate::internal::{synchronize_safe_shared, SharedDeliveryQueue};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The left items and the right items, the right ones keyed by the left key that
/// `right_key_selector` maps them to, joined by that key (`FullJoin.Run`): a result exists while
/// either side has an item of its key. A right item replaces another of the same left key.
///
/// Both sources go through one [`SharedDeliveryQueue`], as upstream's; the lock that upstream holds
/// while it subscribes serializes nothing on one thread.
pub(crate) fn full_join<TL, KL, TR, KR, D>(
    left: Rc<dyn IObservable<ChangeSet<TL, KL>>>,
    right: Rc<dyn IObservable<ChangeSet<TR, KR>>>,
    right_key_selector: Rc<dyn Fn(&TR) -> KL>,
    result_selector: Rc<dyn Fn(&KL, Option<&TL>, Option<&TR>) -> D>,
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
        let left_cache = synchronize_safe_shared(left.clone(), &queue).as_observable_cache();
        let right_cache = {
            let right_key_selector = right_key_selector.clone();
            synchronize_safe_shared(right.clone(), &queue).change_key(move |item: &TR| right_key_selector(item)).as_observable_cache()
        };

        // joined is the final cache
        let joined_cache = Rc::new(RefCell::new(ChangeAwareCache::<D, KL>::new()));

        let left_loader = {
            let (joined_cache, right_cache, result_selector) = (joined_cache.clone(), right_cache.clone(), result_selector.clone());
            left_cache.connect().select(move |changes: ChangeSet<TL, KL>| {
                let mut joined_cache = joined_cache.borrow_mut();
                for change in &changes {
                    let left_current = change.current();
                    let right_lookup = right_cache.lookup(change.key());

                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => {
                            let destination = result_selector(change.key(), Some(left_current), right_lookup.as_ref());
                            joined_cache.add_or_update(destination, change.key().clone());
                        }
                        ChangeReason::Remove => match &right_lookup {
                            // remove from result because there is no left and no rights
                            None => joined_cache.remove(change.key()),
                            // update with no left value
                            Some(right) => {
                                let destination = result_selector(change.key(), None, Some(right));
                                joined_cache.add_or_update(destination, change.key().clone());
                            }
                        },
                        // propagate upstream
                        ChangeReason::Refresh => joined_cache.refresh(change.key()),
                        ChangeReason::Moved => {}
                    }
                }
                joined_cache.capture_changes()
            })
        };

        let right_loader = {
            let (joined_cache, left_cache, result_selector) = (joined_cache.clone(), left_cache.clone(), result_selector.clone());
            right_cache.connect().select(move |changes: ChangeSet<TR, KL>| {
                let mut joined_cache = joined_cache.borrow_mut();
                for change in &changes {
                    let right = change.current();
                    let left = left_cache.lookup(change.key());

                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => {
                            let destination = result_selector(change.key(), left.as_ref(), Some(right));
                            joined_cache.add_or_update(destination, change.key().clone());
                        }
                        ChangeReason::Remove => match &left {
                            // remove from result because there is no left and no rights
                            None => joined_cache.remove(change.key()),
                            // update with no right value
                            Some(left) => {
                                let destination = result_selector(change.key(), Some(left), None);
                                joined_cache.add_or_update(destination, change.key().clone());
                            }
                        },
                        // propagate upstream
                        ChangeReason::Refresh => joined_cache.refresh(change.key()),
                        ChangeReason::Moved => {}
                    }
                }
                joined_cache.capture_changes()
            })
        };

        let disposables: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([
            left_loader.merge(right_loader).subscribe_safe(observer),
            Rc::new(left_cache) as Rc<dyn IDisposable>,
            Rc::new(right_cache) as Rc<dyn IDisposable>,
            queue as Rc<dyn IDisposable>,
        ]);
        disposables
    })
}
