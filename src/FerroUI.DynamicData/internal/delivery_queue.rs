//! `DeliveryQueue<T>` (`Internal/DeliveryQueue.cs`): notifications enqueued while the queue is
//! held, delivered one at a time outside of the hold.
//!
//! Upstream holds a lock while it enqueues and delivers outside of it, on the first thread that
//! releases the lock with notifications pending while nothing is being delivered. On one thread
//! the lock is a scope ([`DeliveryQueue::acquire`]): the scope that ends with notifications
//! pending, while no delivery runs, delivers them; a notification enqueued by an observer during
//! a delivery is delivered after the current one, by the same loop.

use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

/// A notification of the queue (`Internal/Notification.cs`).
enum Notification<T> {
    Next(T),
    Error(ObservableError),
    Completed,
}

impl<T> Notification<T> {
    fn is_terminal(&self) -> bool {
        !matches!(self, Notification::Next(_))
    }

    fn accept(self, observer: &dyn IObserver<T>) {
        match self {
            Notification::Next(value) => observer.on_next(value),
            Notification::Error(error) => observer.on_error(error),
            Notification::Completed => observer.on_completed(),
        }
    }
}

/// The queue (`DeliveryQueue<T>`).
pub(crate) struct DeliveryQueue<T> {
    queue: RefCell<VecDeque<Notification<T>>>,
    observer: Rc<dyn IObserver<T>>,
    /// A delivery runs (`_drainThreadId != -1`).
    delivering: Cell<bool>,
    terminated: Cell<bool>,
}

impl<T: 'static> DeliveryQueue<T> {
    pub(crate) fn new(observer: Rc<dyn IObserver<T>>) -> Rc<Self> {
        Rc::new(Self { queue: RefCell::new(VecDeque::with_capacity(1)), observer, delivering: Cell::new(false), terminated: Cell::new(false) })
    }

    /// Whether a terminal notification has been delivered or the queue disposed (`IsTerminated`).
    pub(crate) fn is_terminated(&self) -> bool {
        self.terminated.get()
    }

    /// Whether notifications are queued or being delivered (`ReadOnlyScopedAccess.HasPending`).
    pub(crate) fn has_pending(&self) -> bool {
        !self.queue.borrow().is_empty() || self.delivering.get()
    }

    /// Holds the queue until the scope ends (`AcquireLock`); the end of the scope delivers.
    pub(crate) fn acquire(self: &Rc<Self>) -> ScopedAccess<T> {
        ScopedAccess { owner: Some(self.clone()) }
    }

    /// Stops the deliveries and drops what is queued (`Dispose`, `EnsureDeliveryComplete`).
    pub(crate) fn dispose(&self) {
        self.terminated.set(true);
        self.queue.borrow_mut().clear();
    }

    fn enqueue(&self, notification: Notification<T>) {
        if self.terminated.get() {
            return;
        }
        self.queue.borrow_mut().push_back(notification);
    }

    /// `ExitLockAndDeliver`.
    fn release(&self) {
        if self.delivering.get() || self.queue.borrow().is_empty() {
            return;
        }
        self.delivering.set(true);
        // The flag is cleared however the delivery ends, a panic of an observer included.
        struct Clear<'a>(&'a Cell<bool>);
        impl Drop for Clear<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _clear = Clear(&self.delivering);
        loop {
            let notification = {
                let mut queue = self.queue.borrow_mut();
                if queue.is_empty() || self.terminated.get() {
                    return;
                }
                let notification = queue.pop_front().expect("the queue is not empty");
                // Marked terminated before the delivery, so that what the observer does sees it.
                if notification.is_terminal() {
                    self.terminated.set(true);
                    queue.clear();
                }
                notification
            };
            let terminal = notification.is_terminal();
            notification.accept(&*self.observer);
            if terminal {
                return;
            }
        }
    }
}

/// A hold of the queue (`ScopedAccess`): what it enqueues is delivered when it ends.
pub(crate) struct ScopedAccess<T: 'static> {
    owner: Option<Rc<DeliveryQueue<T>>>,
}

impl<T: 'static> ScopedAccess<T> {
    pub(crate) fn enqueue_next(&self, value: T) {
        if let Some(owner) = &self.owner {
            owner.enqueue(Notification::Next(value));
        }
    }

    pub(crate) fn enqueue_error(&self, error: ObservableError) {
        if let Some(owner) = &self.owner {
            owner.enqueue(Notification::Error(error));
        }
    }

    pub(crate) fn enqueue_completed(&self) {
        if let Some(owner) = &self.owner {
            owner.enqueue(Notification::Completed);
        }
    }
}

impl<T: 'static> Drop for ScopedAccess<T> {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            // An edit that panics delivers nothing: observers would run during the unwinding.
            if !std::thread::panicking() {
                owner.release();
            }
        }
    }
}
