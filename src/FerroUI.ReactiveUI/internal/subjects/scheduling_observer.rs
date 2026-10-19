//! `SchedulingObserver<T>` (`ReactiveUI.Shared/Internal/Subjects/SchedulingObserver.cs`).

use crate::internal::scheduler_extensions::is_immediate;
use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_reactive::Notification;
use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// Delivers the notifications of an observer on a scheduler, in the order they arrive
/// (`SchedulingObserver<T>`): the first notification schedules a drain of the queue, and the ones
/// that arrive before the drain ends join the queue. On the immediate scheduler they are
/// delivered inline.
pub(crate) struct SchedulingObserver<T> {
    this: Weak<SchedulingObserver<T>>,
    downstream: Rc<dyn IObserver<T>>,
    scheduler: Rc<dyn IScheduler>,
    queue: RefCell<VecDeque<Notification<T>>>,
    draining: Cell<bool>,
}

impl<T: 'static> SchedulingObserver<T> {
    /// An observer that delivers to `downstream` on `scheduler`.
    pub(crate) fn new(downstream: Rc<dyn IObserver<T>>, scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), downstream, scheduler, queue: RefCell::new(VecDeque::new()), draining: Cell::new(false) })
    }

    fn enqueue(&self, notification: Notification<T>) {
        // The immediate scheduler runs work inline on the calling thread, so arrival order is
        // already kept.
        if is_immediate(&self.scheduler) {
            self.deliver(notification);
            return;
        }
        self.queue.borrow_mut().push_back(notification);
        if self.draining.replace(true) {
            return;
        }
        // The scheduled drain holds the observer strongly, as upstream's closure does: a subject
        // forgets its observers once it ends, so a weak reference would lose the terminal
        // notification that is still queued.
        let Some(this) = self.this.upgrade() else { return };
        let _ = self.scheduler.schedule_action(move || this.drain());
    }

    fn drain(&self) {
        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some(notification) = next else {
                self.draining.set(false);
                return;
            };
            self.deliver(notification);
        }
    }

    fn deliver(&self, notification: Notification<T>) {
        match notification {
            Notification::OnNext(value) => self.downstream.on_next(value),
            Notification::OnError(error) => self.downstream.on_error(error),
            Notification::OnCompleted => self.downstream.on_completed(),
        }
    }
}

impl<T: 'static> IObserver<T> for SchedulingObserver<T> {
    fn on_next(&self, value: T) {
        self.enqueue(Notification::OnNext(value));
    }

    fn on_error(&self, error: ObservableError) {
        self.enqueue(Notification::OnError(error));
    }

    fn on_completed(&self) {
        self.enqueue(Notification::OnCompleted);
    }
}
