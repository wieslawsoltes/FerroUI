//! `InitialBindingSignal`.

use ferroui_reactive::disposables::SerialDisposable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// Wires both directions of a binding before it delivers the initial signal of the view model
/// (`InitialBindingSignal`).
///
/// The source is the merged direction signals of a triggered binding (`true` for the view model,
/// `false` for the view). The values the source produces while it is being subscribed (the current
/// values of both sides) are dropped; once it is attached, one `true` is delivered, and the
/// notifications raised meanwhile are delivered after it, one at a time. A terminal notification
/// releases the subscription to the source.
pub struct InitialBindingSignal {
    source: Rc<dyn IObservable<bool>>,
}

impl InitialBindingSignal {
    /// The signal over `source`, the combined direction signals.
    pub fn new(source: Rc<dyn IObservable<bool>>) -> Self {
        Self { source }
    }
}

impl IObservable<bool> for InitialBindingSignal {
    fn subscribe(&self, observer: Rc<dyn IObserver<bool>>) -> Rc<dyn IDisposable> {
        let subscription = Rc::new_cyclic(|this| Subscription {
            this: this.clone(),
            observer,
            upstream: SerialDisposable::new(),
            ready: Cell::new(false),
            pending: RefCell::new(VecDeque::new()),
            draining: Cell::new(false),
            terminal_requested: Cell::new(false),
            stopped: Cell::new(false),
        });
        subscription.start(&self.source);
        subscription
    }
}

/// A notification waiting for the delivery running.
enum Pending {
    Next(bool),
    Error(ObservableError),
    Completed,
}

/// Drops the snapshots of the attachment and serializes the updates behind the initial signal.
struct Subscription {
    this: Weak<Subscription>,
    observer: Rc<dyn IObserver<bool>>,
    /// The subscription to the combined notifications.
    upstream: Rc<SerialDisposable>,
    /// Whether both directions are attached.
    ready: Cell<bool>,
    pending: RefCell<VecDeque<Pending>>,
    /// Whether a delivery runs (or is reserved for the initial signal).
    draining: Cell<bool>,
    /// Whether a terminal notification was received.
    terminal_requested: Cell<bool>,
    /// Whether the subscription was disposed, or delivered its terminal notification.
    stopped: Cell<bool>,
}

/// Leaves the delivery when it ends, a panic included.
struct DrainExit<'a>(&'a Cell<bool>);

impl Drop for DrainExit<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// Disposes the subscription when its start panics, as upstream disposes it in a `catch` before
/// rethrowing; emptied once it started.
struct DisposeOnUnwind(Option<Rc<Subscription>>);

impl Drop for DisposeOnUnwind {
    fn drop(&mut self) {
        if let Some(subscription) = self.0.take() {
            subscription.dispose();
        }
    }
}

impl Subscription {
    /// Reserves the initial delivery while the source is attached, then delivers the initial
    /// signal and what was raised meanwhile (`Start`).
    fn start(&self, source: &Rc<dyn IObservable<bool>>) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let mut guard = DisposeOnUnwind(Some(this.clone()));
        self.draining.set(true);
        let attached = source.subscribe(this);
        self.upstream.set_disposable(Some(attached));
        self.ready.set(true);
        {
            let _exit = DrainExit(&self.draining);
            if !self.stopped.get() {
                self.observer.on_next(true);
            }
            self.drain_claimed();
        }
        guard.0 = None;
    }

    /// Queues `notification` and delivers it unless a delivery runs, which delivers it next.
    fn enqueue(&self, notification: Pending) {
        self.pending.borrow_mut().push_back(notification);
        if self.draining.replace(true) {
            return;
        }
        let _exit = DrainExit(&self.draining);
        self.drain_claimed();
    }

    /// Delivers the waiting notifications in order, with the delivery reserved; a terminal one
    /// releases the source (`DrainTo`).
    fn drain_claimed(&self) {
        loop {
            if self.stopped.get() {
                self.pending.borrow_mut().clear();
                return;
            }
            let Some(next) = self.pending.borrow_mut().pop_front() else {
                return;
            };
            match next {
                Pending::Next(value) => self.observer.on_next(value),
                Pending::Error(error) => {
                    self.stop();
                    self.observer.on_error(error);
                    self.upstream.dispose();
                    return;
                }
                Pending::Completed => {
                    self.stop();
                    self.observer.on_completed();
                    self.upstream.dispose();
                    return;
                }
            }
        }
    }

    fn stop(&self) {
        self.stopped.set(true);
        self.pending.borrow_mut().clear();
    }
}

impl IObserver<bool> for Subscription {
    fn on_next(&self, value: bool) {
        if self.ready.get() && !self.stopped.get() && !self.terminal_requested.get() {
            self.enqueue(Pending::Next(value));
        }
    }

    fn on_error(&self, error: ObservableError) {
        if self.stopped.get() || self.terminal_requested.replace(true) {
            return;
        }
        self.enqueue(Pending::Error(error));
    }

    fn on_completed(&self) {
        if self.stopped.get() || self.terminal_requested.replace(true) {
            return;
        }
        self.enqueue(Pending::Completed);
    }
}

impl IDisposable for Subscription {
    fn dispose(&self) {
        self.stop();
        self.upstream.dispose();
    }
}
