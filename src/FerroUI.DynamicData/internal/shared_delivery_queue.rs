//! `SharedDeliveryQueue` and `DeliverySubQueue<T>` (`Internal/SharedDeliveryQueue.cs`): one queue
//! of delivery for several sources of different types, each with a typed sub-queue, delivered in
//! the order the notifications were received.
//!
//! Upstream holds a lock while it enqueues and delivers outside of it, on the first thread that
//! finds notifications pending while nothing is being delivered. A notification raised by an
//! observer on the delivering thread is delivered inline, before that observer returns, unless its
//! source is the one being delivered (an observer is never re-entered): then it waits its turn.
//!
//! Deviation (DEVIATIONS.md, DynamicData): on one thread the lock is a scope
//! ([`DeliverySubQueue::acquire`]), and every re-entrant
//! notification is one of the delivering thread, so the algorithm keeps its two paths: the drain
//! loop of the scope that found the queue idle, and the inline delivery of a source that
//! re-enters it.
#![allow(dead_code)]

use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// A notification of a sub-queue (`Internal/Notification.cs`).
enum Notification<T> {
    Next(T),
    Error(ObservableError),
    Completed,
}

impl<T> Notification<T> {
    fn is_error(&self) -> bool {
        matches!(self, Notification::Error(_))
    }

    fn accept(self, observer: &dyn IObserver<T>) {
        match self {
            Notification::Next(value) => observer.on_next(value),
            Notification::Error(error) => observer.on_error(error),
            Notification::Completed => observer.on_completed(),
        }
    }
}

/// A sub-queue as the drain loop holds it, without its type (`DrainableBase`).
trait Drainable {
    /// Whether the staged notification is an error (`IsStagedError`).
    fn is_staged_error(&self) -> bool;
    /// How many pending notifications a re-entrant delivery may deliver inline: all of them,
    /// unless this source is the one being delivered (`InlineDeliverableCount`).
    fn inline_deliverable_count(&self) -> usize;
    /// Stages the next notification for the drain loop, which has taken one of the order entries
    /// of this source; false when the entry belongs to a notification delivered inline already
    /// (`TryStageNext`).
    fn try_stage_next(&self) -> bool;
    /// Stages the next notification for an inline delivery, ahead of its order entry
    /// (`TryStageInline`).
    fn try_stage_inline(&self) -> bool;
    /// Delivers the staged notification (`DeliverStaged`).
    fn deliver_staged(&self);
}

/// The queue shared by several sources (`SharedDeliveryQueue`).
pub(crate) struct SharedDeliveryQueue {
    /// One entry per pending notification, naming its source, in the order received (`_order`).
    order: RefCell<VecDeque<Rc<dyn Drainable>>>,
    /// A drain loop runs (`_drainThreadId != -1`).
    draining: Cell<bool>,
    terminated: Cell<bool>,
}

/// Clears the flag of a drain loop however the loop ends, a panic of an observer included
/// (`ReleaseDrainOwnership` in the `catch` of `DrainAll`).
struct ReleaseDrain<'a>(&'a Cell<bool>);

impl Drop for ReleaseDrain<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl SharedDeliveryQueue {
    /// An empty queue (`SharedDeliveryQueue()`; the form with a caller's lock is the same on one
    /// thread).
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self { order: RefCell::new(VecDeque::new()), draining: Cell::new(false), terminated: Cell::new(false) })
    }

    /// Whether the queue has been terminated, by its disposal or by an error (`IsTerminated`).
    pub(crate) fn is_terminated(&self) -> bool {
        self.terminated.get()
    }

    /// Whether a notification is pending or being delivered (`ReadOnlyScopedAccess.HasPending`).
    pub(crate) fn has_pending(&self) -> bool {
        self.draining.get() || !self.order.borrow().is_empty()
    }

    /// A typed sub-queue that delivers to `observer` (`CreateQueue<T>`).
    pub(crate) fn create_queue<T: 'static>(self: &Rc<Self>, observer: Rc<dyn IObserver<T>>) -> Rc<DeliverySubQueue<T>> {
        Rc::new_cyclic(|this| DeliverySubQueue {
            this: this.clone(),
            items: RefCell::new(VecDeque::with_capacity(1)),
            parent: self.clone(),
            observer,
            staged: RefCell::new(None),
            inline_delivered_count: Cell::new(0),
            is_delivering: Cell::new(false),
            is_removed: Cell::new(false),
        })
    }

    /// Terminates the queue and drops what is pending (`Dispose`); a delivery that runs sees the
    /// termination and stops after its current notification.
    pub(crate) fn dispose(&self) {
        self.terminated.set(true);
        let pending = std::mem::take(&mut *self.order.borrow_mut());
        drop(pending);
    }

    fn enqueue_order(&self, source: Rc<dyn Drainable>) {
        self.order.borrow_mut().push_back(source);
    }

    /// Delivers after `source` has enqueued (`ExitLockAndDrain`).
    fn exit_lock_and_drain(&self, source: Rc<dyn Drainable>) {
        // Re-entrant: an observer raised the notification while one was being delivered, so it is
        // delivered inline, as `Synchronize(lock)` re-entrancy did. Only this source is delivered:
        // the head of the order queue waits for the delivery that runs.
        if self.draining.get() {
            let inline_count = if self.terminated.get() { 0 } else { source.inline_deliverable_count() };
            self.deliver_inline(&*source, inline_count);
            return;
        }
        if !self.terminated.get() && !self.order.borrow().is_empty() {
            self.draining.set(true);
            self.drain_all();
        }
    }

    /// `DrainAll`.
    fn drain_all(&self) {
        let _release = ReleaseDrain(&self.draining);
        loop {
            if !self.drain_pending() {
                return;
            }
            if !self.order.borrow().is_empty() && !self.terminated.get() {
                continue;
            }
            return;
        }
    }

    /// Delivers the pending notifications one at a time, in the order received; false when the
    /// queue was terminated (`DrainPending`).
    fn drain_pending(&self) -> bool {
        loop {
            if self.terminated.get() {
                return false;
            }
            let source = match self.order.borrow_mut().pop_front() {
                Some(source) => source,
                None => return true,
            };
            // The entry is stale when its notification was delivered inline already, or when its
            // source has been disposed since: skip it.
            if !source.try_stage_next() {
                continue;
            }
            if !self.deliver_staged(&*source) {
                return false;
            }
        }
    }

    /// Delivers the oldest `count` pending notifications of one source, for a notification it
    /// raised re-entrantly (`DeliverInline`).
    fn deliver_inline(&self, source: &dyn Drainable, count: usize) {
        for _ in 0..count {
            if self.terminated.get() || !source.try_stage_inline() {
                return;
            }
            if !self.deliver_staged(source) {
                return;
            }
        }
    }

    /// Delivers the staged notification of `source`, and terminates the queue when it was an error;
    /// false when the queue was terminated (`ExitLockAndDeliverStaged`).
    fn deliver_staged(&self, source: &dyn Drainable) -> bool {
        let is_error = source.is_staged_error();
        source.deliver_staged();
        if !is_error {
            return true;
        }
        self.terminated.set(true);
        let pending = std::mem::take(&mut *self.order.borrow_mut());
        drop(pending);
        false
    }
}

impl IDisposable for SharedDeliveryQueue {
    fn dispose(&self) {
        SharedDeliveryQueue::dispose(self);
    }
}

/// A typed sub-queue of a [`SharedDeliveryQueue`] (`DeliverySubQueue<T>`): the notifications of one
/// source, delivered to its observer.
pub(crate) struct DeliverySubQueue<T> {
    this: Weak<DeliverySubQueue<T>>,
    items: RefCell<VecDeque<Notification<T>>>,
    parent: Rc<SharedDeliveryQueue>,
    observer: Rc<dyn IObserver<T>>,
    staged: RefCell<Option<Notification<T>>>,
    inline_delivered_count: Cell<usize>,
    is_delivering: Cell<bool>,
    is_removed: Cell<bool>,
}

impl<T: 'static> DeliverySubQueue<T> {
    /// Holds the parent queue until the scope ends (`AcquireLock`); the end of the scope delivers.
    pub(crate) fn acquire(&self) -> SubQueueScopedAccess<T> {
        SubQueueScopedAccess { owner: self.this.upgrade() }
    }

    /// Drops the pending notifications of this source and stops its deliveries; its entries in
    /// the order queue are skipped when the drain reaches them (`Dispose`). Idempotent.
    pub(crate) fn dispose(&self) {
        if self.is_removed.replace(true) {
            return;
        }
        let pending = std::mem::take(&mut *self.items.borrow_mut());
        drop(pending);
    }

    fn try_stage(&self) -> bool {
        if self.is_removed.get() {
            return false;
        }
        let next = self.items.borrow_mut().pop_front();
        match next {
            Some(notification) => {
                *self.staged.borrow_mut() = Some(notification);
                true
            }
            None => false,
        }
    }

    fn enqueue_item(&self, item: Notification<T>) {
        if self.parent.is_terminated() || self.is_removed.get() {
            return;
        }
        self.items.borrow_mut().push_back(item);
        if let Some(this) = self.this.upgrade() {
            self.parent.enqueue_order(this);
        }
    }
}

/// Clears the flag of a delivery however it ends (`finally { _isDelivering = false; }`).
struct EndDelivery<'a>(&'a Cell<bool>);

impl Drop for EndDelivery<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl<T: 'static> Drainable for DeliverySubQueue<T> {
    fn is_staged_error(&self) -> bool {
        self.staged.borrow().as_ref().is_some_and(Notification::is_error)
    }

    fn inline_deliverable_count(&self) -> usize {
        if self.is_removed.get() || self.is_delivering.get() {
            0
        } else {
            self.items.borrow().len()
        }
    }

    fn try_stage_next(&self) -> bool {
        // An inline delivery takes the oldest notifications of this source, so its oldest order
        // entries are the ones left without a notification.
        let inline_delivered = self.inline_delivered_count.get();
        if inline_delivered != 0 {
            self.inline_delivered_count.set(inline_delivered - 1);
            return false;
        }
        self.try_stage()
    }

    fn try_stage_inline(&self) -> bool {
        if !self.try_stage() {
            return false;
        }
        self.inline_delivered_count.set(self.inline_delivered_count.get() + 1);
        true
    }

    fn deliver_staged(&self) {
        let staged = self.staged.borrow_mut().take();
        self.is_delivering.set(true);
        let _end = EndDelivery(&self.is_delivering);
        if let Some(notification) = staged {
            notification.accept(&*self.observer);
        }
    }
}

impl<T: 'static> IObserver<T> for DeliverySubQueue<T> {
    /// Enqueues the value and delivers (`OnNext`).
    fn on_next(&self, value: T) {
        let scope = self.acquire();
        scope.enqueue_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        let scope = self.acquire();
        scope.enqueue_error(error);
    }

    fn on_completed(&self) {
        let scope = self.acquire();
        scope.enqueue_completed();
    }
}

impl<T: 'static> IDisposable for DeliverySubQueue<T> {
    fn dispose(&self) {
        DeliverySubQueue::dispose(self);
    }
}

/// A hold of the parent queue by a sub-queue (`DeliverySubQueue<T>.ScopedAccess`): what it enqueues
/// is delivered when it ends.
pub(crate) struct SubQueueScopedAccess<T: 'static> {
    owner: Option<Rc<DeliverySubQueue<T>>>,
}

impl<T: 'static> SubQueueScopedAccess<T> {
    pub(crate) fn enqueue_next(&self, value: T) {
        if let Some(owner) = &self.owner {
            owner.enqueue_item(Notification::Next(value));
        }
    }

    pub(crate) fn enqueue_error(&self, error: ObservableError) {
        if let Some(owner) = &self.owner {
            owner.enqueue_item(Notification::Error(error));
        }
    }

    pub(crate) fn enqueue_completed(&self) {
        if let Some(owner) = &self.owner {
            owner.enqueue_item(Notification::Completed);
        }
    }
}

impl<T: 'static> Drop for SubQueueScopedAccess<T> {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            // An edit that panics delivers nothing: observers would run during the unwinding.
            if !std::thread::panicking() {
                let parent = owner.parent.clone();
                parent.exit_lock_and_drain(owner);
            }
        }
    }
}
