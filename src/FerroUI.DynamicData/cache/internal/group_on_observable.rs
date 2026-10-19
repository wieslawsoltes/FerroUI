//! `GroupOnObservable<TObject, TKey, TGroupKey>` (`Cache/Internal/GroupOnObservable.cs`), with the
//! part of `CacheParentSubscription` (`Internal/CacheParentSubscription.cs`) it uses.

use super::dynamic_grouper::DynamicGrouper;
use crate::cache::{ChangeReason, ChangeSet, GroupChangeSet};
use ferroui_reactive::disposables::SingleAssignmentDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::{Rc, Weak};

/// The sequence of the group key of an item, from the item and its key; an error ends the result
/// (`Func<TObject, TKey, IObservable<TGroupKey>>`, which may throw).
pub(crate) type GroupObservableSelector<T, K, G> = Rc<dyn Fn(&T, &K) -> Result<Rc<dyn IObservable<G>>, ObservableError>>;

/// The groups of the items of `source`, each in the group that the latest value of its sequence
/// of group keys names (`GroupOnObservable.Run`). The result ends when the source and the
/// sequences of all its items have ended.
pub(crate) fn group_on_observable<T, K, G>(source: Rc<dyn IObservable<ChangeSet<T, K>>>, select_group: GroupObservableSelector<T, K, G>) -> Rc<dyn IObservable<GroupChangeSet<T, K, G>>>
where
    T: Clone + 'static,
    K: Eq + Hash + Clone + 'static,
    G: Eq + Hash + Clone + 'static,
{
    Observable::create(move |observer: Rc<dyn IObserver<GroupChangeSet<T, K, G>>>| {
        let subscription = Subscription::new(select_group.clone(), observer);
        Subscription::create_parent_subscription(&subscription, &source);
        let disposable: Rc<dyn IDisposable> = subscription;
        disposable
    })
}

/// The state of one subscription (`GroupOnObservable.Subscription`, a
/// `CacheParentSubscription<TObject, TKey, (TGroupKey, TObject), IGroupChangeSet>`): the
/// subscription to the source, one subscription per item to its sequence of group keys, and the
/// frames that emit the changes once the outermost notification ends.
///
/// The `SharedDeliveryQueue` of upstream serializes the source and the children; on one thread
/// they are already serial.
struct Subscription<T, K, G> {
    this: Weak<Self>,
    grouper: RefCell<DynamicGrouper<T, K, G>>,
    select_group: GroupObservableSelector<T, K, G>,
    observer: Rc<dyn IObserver<GroupChangeSet<T, K, G>>>,
    /// The subscriptions of the children by the key of their parent (`KeyedDisposable<TKey>`).
    child_subscriptions: RefCell<Dictionary<K, Rc<SingleAssignmentDisposable>>>,
    parent_subscription: Rc<SingleAssignmentDisposable>,
    /// The subscriptions that have not ended: the parent and the children (`_subscriptionCounter`).
    subscription_counter: Cell<usize>,
    frame_depth: Cell<usize>,
    is_completed: Cell<bool>,
    has_terminated: Cell<bool>,
    disposed: Cell<bool>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G: Eq + Hash + Clone + 'static> Subscription<T, K, G> {
    fn new(select_group: GroupObservableSelector<T, K, G>, observer: Rc<dyn IObserver<GroupChangeSet<T, K, G>>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            grouper: RefCell::new(DynamicGrouper::new(None)),
            select_group,
            observer,
            child_subscriptions: RefCell::new(Dictionary::new()),
            parent_subscription: SingleAssignmentDisposable::new(),
            // It starts at 1, for the subscription to the parent.
            subscription_counter: Cell::new(1),
            frame_depth: Cell::new(0),
            is_completed: Cell::new(false),
            has_terminated: Cell::new(false),
            disposed: Cell::new(false),
        })
    }

    /// `CreateParentSubscription`.
    fn create_parent_subscription(this: &Rc<Self>, source: &Rc<dyn IObservable<ChangeSet<T, K>>>) {
        let (on_next, on_error, on_completed) = (this.this.clone(), this.this.clone(), this.this.clone());
        let subscription = source.subscribe_all(
            move |changes: ChangeSet<T, K>| {
                if let Some(this) = on_next.upgrade() {
                    this.deliver_parent(&changes);
                }
            },
            move |error| {
                if let Some(this) = on_error.upgrade() {
                    this.terminal_error(error);
                }
            },
            move || {
                if let Some(this) = on_completed.upgrade() {
                    this.complete_parent();
                }
            },
        );
        this.parent_subscription.set_disposable(Some(subscription));
    }

    /// `ParentOnNext`: the changes are processed together, to keep the order of the change set.
    fn parent_on_next(&self, changes: &ChangeSet<T, K>) {
        for change in changes {
            self.grouper.borrow_mut().process_change(change);

            match change.reason() {
                // End the subscription of the item, if any, and make one that updates its group key.
                ChangeReason::Add | ChangeReason::Update => {
                    if let Err(error) = self.add_group_subscription(change.current().clone(), change.key().clone()) {
                        self.terminal_error(error);
                        return;
                    }
                }
                // End the subscription of the item.
                ChangeReason::Remove => self.remove_child_subscription(change.key()),
                ChangeReason::Refresh | ChangeReason::Moved => {}
            }
        }
    }

    /// `ChildOnNext`.
    fn child_on_next(&self, (group_key, item): (G, T), parent_key: K) {
        // The grouper does not emit here, so it suspends the groups it changes.
        let emission = self.grouper.borrow_mut().add_or_update(parent_key, group_key, item, false);
        debug_assert!(emission.is_none());
    }

    /// `AddGroupSubscription`.
    fn add_group_subscription(&self, item: T, key: K) -> Result<(), ObservableError> {
        let group_keys = (self.select_group)(&item, &key)?;
        let child = group_keys.distinct_until_changed().select(move |group_key: G| (group_key, item.clone()));
        self.add_child_subscription(child, key);
        Ok(())
    }

    /// `AddChildSubscription`.
    fn add_child_subscription(&self, observable: Rc<dyn IObservable<(G, T)>>, parent_key: K) {
        // Count the new subscription first, so that ending the one it replaces does not end the
        // result.
        self.subscription_counter.set(self.subscription_counter.get() + 1);

        // The container of the subscription, which replaces (and disposes) that of the key.
        let container = SingleAssignmentDisposable::new();
        let replaced = self.child_subscriptions.borrow_mut().insert(parent_key.clone(), container.clone());
        if let Some(replaced) = replaced {
            if !Rc::ptr_eq(&replaced, &container) {
                replaced.dispose();
            }
        }

        // The end of the child, by completion, error or disposal, counts down the subscriptions;
        // its completion removes its subscription, which is no longer needed. A child that
        // completes on subscription disposes its container before it is assigned, which disposes
        // the subscription at once.
        let (finally, on_next, on_error, on_completed) = (self.this.clone(), self.this.clone(), self.this.clone(), self.this.clone());
        let (next_key, completed_key) = (parent_key.clone(), parent_key);
        let subscription = observable
            .finally(move || {
                if let Some(this) = finally.upgrade() {
                    this.check_completed();
                }
            })
            .subscribe_all(
                move |value: (G, T)| {
                    if let Some(this) = on_next.upgrade() {
                        this.deliver_child(value, next_key.clone());
                    }
                },
                move |error| {
                    if let Some(this) = on_error.upgrade() {
                        this.terminal_error(error);
                    }
                },
                move || {
                    if let Some(this) = on_completed.upgrade() {
                        this.complete_child(&completed_key);
                    }
                },
            );
        container.set_disposable(Some(subscription));
    }

    /// `RemoveChildSubscription`.
    fn remove_child_subscription(&self, parent_key: &K) {
        let removed = self.child_subscriptions.borrow_mut().remove(parent_key);
        if let Some(removed) = removed {
            removed.dispose();
        }
    }

    fn deliver_parent(&self, changes: &ChangeSet<T, K>) {
        self.begin_frame();
        self.parent_on_next(changes);
        self.end_frame();
    }

    fn deliver_child(&self, value: (G, T), parent_key: K) {
        self.begin_frame();
        self.child_on_next(value, parent_key);
        self.end_frame();
    }

    fn complete_parent(&self) {
        self.begin_frame();
        self.check_completed();
        self.end_frame();
    }

    fn complete_child(&self, parent_key: &K) {
        self.begin_frame();
        self.remove_child_subscription(parent_key);
        self.end_frame();
    }

    fn begin_frame(&self) {
        self.frame_depth.set(self.frame_depth.get() + 1);
    }

    /// `EndFrame`: the outermost frame emits the changes, and the end once every subscription has
    /// ended.
    fn end_frame(&self) {
        let depth = self.frame_depth.get() - 1;
        self.frame_depth.set(depth);
        if depth != 0 {
            return;
        }

        let emission = self.grouper.borrow_mut().emit_changes();
        emission.deliver(&*self.observer);

        if self.is_completed.get() && !self.has_terminated.get() {
            self.has_terminated.set(true);
            self.observer.on_completed();
        }
    }

    /// `TerminalError`.
    fn terminal_error(&self, error: ObservableError) {
        self.has_terminated.set(true);
        self.observer.on_error(error);
    }

    /// `CheckCompleted`.
    fn check_completed(&self) {
        let remaining = self.subscription_counter.get().checked_sub(1);
        debug_assert!(remaining.is_some(), "Should never be negative");
        let remaining = remaining.unwrap_or(0);
        self.subscription_counter.set(remaining);
        if remaining == 0 {
            self.is_completed.set(true);
        }
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, G: Eq + Hash + Clone + 'static> IDisposable for Subscription<T, K, G> {
    /// `Dispose`: the grouper (its suspensions and groups), then the subscription to the source and
    /// those of the children.
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let (suspensions, groups) = self.grouper.borrow_mut().take_for_dispose();
        for suspension in suspensions {
            suspension.dispose();
        }
        for group in groups {
            group.dispose();
        }
        self.parent_subscription.dispose();
        let children: Vec<Rc<SingleAssignmentDisposable>> = self.child_subscriptions.borrow().values().cloned().collect();
        self.child_subscriptions.borrow_mut().clear();
        for child in children {
            child.dispose();
        }
    }
}
