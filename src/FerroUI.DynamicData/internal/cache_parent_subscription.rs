//! `CacheParentSubscription<TParent, TKey, TChild, TObserver>`
//! (`Internal/CacheParentSubscription.cs`): the subscription of an operator that subscribes to a
//! child sequence for each item of a parent cache (`MergeManyChangeSets` and its siblings).
//!
//! The parent and the children are delivered through one [`SharedDeliveryQueue`], so that their
//! notifications never interleave. Each delivery is a frame; a child delivered inline during the
//! delivery of its parent is a frame within it. When the outermost frame ends, the operator emits
//! what its frames changed ([`ParentSubscriptionHandler::emit_changes`]), and the completion once
//! the parent and every child have completed.
//!
//! Upstream's abstract class becomes a struct with the handler of the operator: the three abstract
//! methods are those of [`ParentSubscriptionHandler`], and the protected methods
//! (`AddChildSubscription`, `RemoveChildSubscription`, `CreateParentSubscription`,
//! `MakeChildObservable`) are methods of the struct, which the handler receives.
#![allow(dead_code)]

use super::keyed_disposable::KeyedDisposable;
use super::shared_delivery_queue::SharedDeliveryQueue;
use super::synchronize_safe::synchronize_safe_shared;
use crate::cache::ChangeSet;
use ferroui_reactive::disposables::SingleAssignmentDisposable;
use ferroui_reactive::disposables::SingleAssignmentDisposableValue;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::hash::Hash;
use std::rc::{Rc, Weak};

/// What an operator does with the deliveries of its parent and its children (the abstract methods
/// of `CacheParentSubscription`).
pub(crate) trait ParentSubscriptionHandler<P, K, C, O> {
    /// A change set of the parent (`ParentOnNext`); the handler adds and removes the child
    /// subscriptions through `owner`.
    fn parent_on_next(&self, owner: &CacheParentSubscription<P, K, C, O>, changes: ChangeSet<P, K>);
    /// A value of the child of `parent_key` (`ChildOnNext`).
    fn child_on_next(&self, child: C, parent_key: &K);
    /// Emits what the frames changed, when the outermost ends (`EmitChanges`).
    fn emit_changes(&self, observer: &Rc<dyn IObserver<O>>);
}

/// The subscription (`CacheParentSubscription<TParent, TKey, TChild, TObserver>`).
pub(crate) struct CacheParentSubscription<P, K, C, O> {
    this: Weak<CacheParentSubscription<P, K, C, O>>,
    child_subscriptions: KeyedDisposable<K>,
    parent_subscription: SingleAssignmentDisposableValue,
    queue: Rc<SharedDeliveryQueue>,
    observer: Rc<dyn IObserver<O>>,
    /// The parent and each child subscription that has not ended; one for the parent at first.
    subscription_counter: Cell<isize>,
    frame_depth: Cell<usize>,
    is_completed: Cell<bool>,
    has_terminated: Cell<bool>,
    disposed: Cell<bool>,
    handler: Box<dyn ParentSubscriptionHandler<P, K, C, O>>,
}

impl<P: Clone + 'static, K: Eq + Hash + Clone + 'static, C: 'static, O: 'static> CacheParentSubscription<P, K, C, O> {
    /// A subscription that emits to `observer` (`CacheParentSubscription(IObserver<TObserver>)`);
    /// [`create_parent_subscription`](Self::create_parent_subscription) then subscribes it to the
    /// parent.
    pub(crate) fn new(observer: Rc<dyn IObserver<O>>, handler: Box<dyn ParentSubscriptionHandler<P, K, C, O>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            child_subscriptions: KeyedDisposable::new(),
            parent_subscription: SingleAssignmentDisposableValue::new(),
            queue: SharedDeliveryQueue::new(),
            observer,
            subscription_counter: Cell::new(1),
            frame_depth: Cell::new(0),
            is_completed: Cell::new(false),
            has_terminated: Cell::new(false),
            disposed: Cell::new(false),
            handler,
        })
    }

    /// The handler of the operator.
    pub(crate) fn handler(&self) -> &dyn ParentSubscriptionHandler<P, K, C, O> {
        &*self.handler
    }

    /// The queue that serializes the parent and the children.
    pub(crate) fn queue(&self) -> &Rc<SharedDeliveryQueue> {
        &self.queue
    }

    fn strong(&self) -> Rc<Self> {
        self.this.upgrade().expect("the subscription is alive while it is used")
    }

    /// Subscribes to `observable`, the child of `parent_key`, disposing the subscription it
    /// replaces (`AddChildSubscription`). The child is counted first, so that the disposal of the
    /// replaced one does not complete the operator; a child that completes is removed, and its
    /// subscription disposed, at once.
    pub(crate) fn add_child_subscription(&self, observable: Rc<dyn IObservable<C>>, parent_key: K) {
        self.subscription_counter.set(self.subscription_counter.get() + 1);
        let container = SingleAssignmentDisposable::new();
        self.child_subscriptions.add(parent_key.clone(), Some(container.clone()));
        // `Finally` runs at the completion, at the error and at the disposal, so the counter always
        // goes down; the removal of the child happens at its completion only.
        let (finally_owner, next_owner, error_owner, completed_owner) = (self.strong(), self.strong(), self.strong(), self.strong());
        let (next_key, completed_key) = (parent_key.clone(), parent_key);
        let subscription = observable.finally(move || finally_owner.check_completed()).subscribe_all(
            move |value| next_owner.deliver_child(value, &next_key),
            move |error| error_owner.terminal_error(error),
            move || completed_owner.complete_child(&completed_key),
        );
        container.set_disposable(Some(subscription));
    }

    /// Disposes and forgets the child subscription of `parent_key` (`RemoveChildSubscription`).
    pub(crate) fn remove_child_subscription(&self, parent_key: &K) {
        self.child_subscriptions.remove(parent_key);
    }

    /// Subscribes to the parent, through the queue (`CreateParentSubscription`).
    pub(crate) fn create_parent_subscription(&self, source: Rc<dyn IObservable<ChangeSet<P, K>>>) {
        let (next_owner, error_owner, completed_owner) = (self.strong(), self.strong(), self.strong());
        let subscription = synchronize_safe_shared(source, &self.queue).subscribe_all(
            move |changes| next_owner.deliver_parent(changes),
            move |error| error_owner.terminal_error(error),
            move || completed_owner.complete_parent(),
        );
        self.parent_subscription.set_disposable(Some(subscription));
    }

    /// A child sequence delivered through the queue of the subscription (`MakeChildObservable`).
    pub(crate) fn make_child_observable<T: 'static>(&self, observable: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
        synchronize_safe_shared(observable, &self.queue)
    }

    fn deliver_parent(&self, changes: ChangeSet<P, K>) {
        self.begin_frame();
        self.handler.parent_on_next(self, changes);
        self.end_frame();
    }

    fn deliver_child(&self, child: C, parent_key: &K) {
        self.begin_frame();
        self.handler.child_on_next(child, parent_key);
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

    fn end_frame(&self) {
        let depth = self.frame_depth.get() - 1;
        self.frame_depth.set(depth);
        if depth != 0 {
            return;
        }
        self.handler.emit_changes(&self.observer);
        if self.is_completed.get() && !self.has_terminated.get() {
            self.has_terminated.set(true);
            self.observer.on_completed();
        }
    }

    fn terminal_error(&self, error: ObservableError) {
        self.has_terminated.set(true);
        self.observer.on_error(error);
    }

    fn check_completed(&self) {
        let counter = self.subscription_counter.get() - 1;
        self.subscription_counter.set(counter);
        if counter == 0 {
            self.is_completed.set(true);
        }
        debug_assert!(counter >= 0, "Should never be negative");
    }
}

impl<P, K: Eq + Hash + Clone, C, O> IDisposable for CacheParentSubscription<P, K, C, O> {
    /// Disposes the queue, then the parent subscription, then the children (`Dispose`).
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.queue.dispose();
        self.parent_subscription.dispose();
        self.child_subscriptions.dispose();
    }
}
