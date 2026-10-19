//! `TransformOnObservable<TSource, TKey, TDestination>` (`Cache/Internal/TransformOnObservable.cs`).

use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache};
use crate::internal::{CacheParentSubscription, ParentSubscriptionHandler};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The observable of an item of the source and its key (`Func<TSource, TKey, IObservable<TDestination>>`).
pub(crate) type ObservableTransform<S, K, D> = Rc<dyn Fn(&S, &K) -> Rc<dyn IObservable<D>>>;

/// For each item of the source, the latest distinct value of the observable `transform` makes of
/// it, under the key of the item (`TransformOnObservable.Run`).
///
/// An addition or an update subscribes to a new observable, ending the one of the key; a removal
/// ends it and removes the value; a refresh is forwarded, or with `transform_on_refresh`
/// subscribes again. The changes that one delivery of the source makes, the values that its
/// observables produce at once included, come as one change set. The sequence ends when the
/// source and every observable have ended, and fails with the first error of any of them.
pub(crate) fn transform_on_observable<S: Clone + 'static, K: Eq + Hash + Clone + 'static, D: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, K>>>,
    transform: ObservableTransform<S, K, D>,
    transform_on_refresh: bool,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D, K>>>| {
        let handler = Subscription { cache: RefCell::new(ChangeAwareCache::new()), transform: transform.clone(), transform_on_refresh };
        let subscription = CacheParentSubscription::new(observer, Box::new(handler));
        subscription.create_parent_subscription(source.clone());
        let subscription: Rc<dyn IDisposable> = subscription;
        subscription
    })
}

/// The state of one subscription (`TransformOnObservable.Subscription`).
struct Subscription<S, K, D> {
    cache: RefCell<ChangeAwareCache<D, K>>,
    transform: ObservableTransform<S, K, D>,
    transform_on_refresh: bool,
}

impl<S: Clone + 'static, K: Eq + Hash + Clone + 'static, D: Clone + PartialEq + 'static> Subscription<S, K, D> {
    /// Ends the observable of `key`, if any, and subscribes to the one of `item`
    /// (`AddTransformSubscription`).
    fn add_transform_subscription(&self, owner: &CacheParentSubscription<S, K, D, ChangeSet<D, K>>, item: &S, key: &K) {
        let observable = (self.transform)(item, key).distinct_until_changed();
        owner.add_child_subscription(owner.make_child_observable(observable), key.clone());
    }
}

impl<S: Clone + 'static, K: Eq + Hash + Clone + 'static, D: Clone + PartialEq + 'static> ParentSubscriptionHandler<S, K, D, ChangeSet<D, K>> for Subscription<S, K, D> {
    /// `ParentOnNext`: all the changes at once, to keep the order of the change set.
    fn parent_on_next(&self, owner: &CacheParentSubscription<S, K, D, ChangeSet<D, K>>, changes: ChangeSet<S, K>) {
        for change in &changes {
            match change.reason() {
                // End the subscription of the key (if any) and make a new one, which updates the
                // cache and emits the changes.
                ChangeReason::Add | ChangeReason::Update => self.add_transform_subscription(owner, change.current(), change.key()),
                // End the subscription and remove the value from the cache.
                ChangeReason::Remove => {
                    self.cache.borrow_mut().remove(change.key());
                    owner.remove_child_subscription(change.key());
                }
                ChangeReason::Refresh => {
                    if self.transform_on_refresh {
                        self.add_transform_subscription(owner, change.current(), change.key());
                    } else {
                        // Let the downstream decide what this means.
                        self.cache.borrow_mut().refresh(change.key());
                    }
                }
                ChangeReason::Moved => {}
            }
        }
    }

    /// `ChildOnNext`.
    fn child_on_next(&self, child: D, parent_key: &K) {
        self.cache.borrow_mut().add_or_update(child, parent_key.clone());
    }

    /// `EmitChanges`: the changes of the frames, when there are any.
    fn emit_changes(&self, observer: &Rc<dyn IObserver<ChangeSet<D, K>>>) {
        let changes = self.cache.borrow_mut().capture_changes();
        if changes.count() > 0 {
            observer.on_next(changes);
        }
    }
}
