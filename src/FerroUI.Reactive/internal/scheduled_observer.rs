//! `IScheduledObserver<T>`, `ScheduledObserver<T>` (`Internal/ScheduledObserver.cs`) and
//! `FastImmediateObserver<T>` (`Subjects/ReplaySubject.cs`): observers that queue the notifications
//! they receive and deliver them when they are made active.

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::SerialDisposableValue;
use crate::notification::Notification;
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// An observer that queues notifications until it is made active (`IScheduledObserver<T>`).
pub(crate) trait IScheduledObserver<T>: IObserver<T> + IDisposable {
    /// Delivers the queued notifications (`EnsureActive()`).
    fn ensure_active(&self) {
        self.ensure_active_n(1);
    }

    /// Delivers the queued notifications when `n` of them were queued (`EnsureActive(int)`).
    fn ensure_active_n(&self, n: usize);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RunState {
    Stopped,
    Running,
    Pending,
    Faulted,
}

/// Delivers the notifications it queues on a scheduler, one value per scheduled step
/// (`ScheduledObserver<T>`), in the order they were received; the terminal notification follows
/// the last value.
pub(crate) struct ScheduledObserver<T> {
    this: Weak<ScheduledObserver<T>>,
    observer: Rc<dyn IObserver<T>>,
    scheduler: Rc<dyn IScheduler>,
    queue: RefCell<VecDeque<T>>,
    state: Cell<RunState>,
    failed: RefCell<Option<ObservableError>>,
    completed: Cell<bool>,
    is_stopped: Cell<bool>,
    disposable: SerialDisposableValue,
}

impl<T: 'static> ScheduledObserver<T> {
    pub(crate) fn new(scheduler: Rc<dyn IScheduler>, observer: Rc<dyn IObserver<T>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            observer,
            scheduler,
            queue: RefCell::new(VecDeque::new()),
            state: Cell::new(RunState::Stopped),
            failed: RefCell::new(None),
            completed: Cell::new(false),
            is_stopped: Cell::new(false),
            disposable: SerialDisposableValue::new(),
        })
    }

    fn ensure_active_slow(&self) {
        match self.state.get() {
            RunState::Stopped => {
                self.state.set(RunState::Running);
                let this = self.this.clone();
                let run = self.scheduler.schedule_recursive(move |recurse| {
                    if let Some(this) = this.upgrade() {
                        this.run(recurse);
                    }
                });
                self.disposable.set_disposable(Some(run));
            }
            RunState::Running => self.state.set(RunState::Pending),
            RunState::Pending | RunState::Faulted => {}
        }
    }

    /// One step of the delivery (`Run`): one value, then a recursion; the terminal notification
    /// once the queue is empty.
    fn run(&self, recurse: &dyn Fn()) {
        let next = loop {
            let next = self.queue.borrow_mut().pop_front();
            if let Some(next) = next {
                break next;
            }
            let failed = self.failed.borrow().clone();
            if let Some(error) = failed {
                self.state.set(RunState::Stopped);
                self.observer.on_error(error);
                self.dispose();
                return;
            }
            if self.completed.get() {
                self.state.set(RunState::Stopped);
                self.observer.on_completed();
                self.dispose();
                return;
            }
            match self.state.get() {
                RunState::Running | RunState::Faulted => {
                    if self.state.get() == RunState::Running {
                        self.state.set(RunState::Stopped);
                    }
                    return;
                }
                RunState::Pending => self.state.set(RunState::Running),
                RunState::Stopped => return,
            }
        };
        self.state.set(RunState::Running);
        struct Fault<'a, T>(&'a ScheduledObserver<T>);
        impl<T> Drop for Fault<'_, T> {
            fn drop(&mut self) {
                if std::thread::panicking() {
                    self.0.state.set(RunState::Faulted);
                    self.0.queue.borrow_mut().clear();
                }
            }
        }
        let fault = Fault(self);
        self.observer.on_next(next);
        drop(fault);
        recurse();
    }
}

impl<T: 'static> IObserver<T> for ScheduledObserver<T> {
    fn on_next(&self, value: T) {
        if !self.is_stopped.get() {
            self.queue.borrow_mut().push_back(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if !self.is_stopped.replace(true) {
            *self.failed.borrow_mut() = Some(error);
        }
    }

    fn on_completed(&self) {
        if !self.is_stopped.replace(true) {
            self.completed.set(true);
        }
    }
}

impl<T: 'static> IDisposable for ScheduledObserver<T> {
    fn dispose(&self) {
        self.is_stopped.set(true);
        self.disposable.dispose();
    }
}

impl<T: 'static> IScheduledObserver<T> for ScheduledObserver<T> {
    fn ensure_active_n(&self, n: usize) {
        if n > 0 {
            self.ensure_active_slow();
        }
    }
}

/// Delivers the notifications it queues at once, on the calling thread, when it is made active
/// (`FastImmediateObserver<T>`); a notification queued while it delivers is delivered by the same
/// loop, after the ones before it.
pub(crate) struct FastImmediateObserver<T> {
    observer: RefCell<Option<Rc<dyn IObserver<T>>>>,
    queue: RefCell<VecDeque<Notification<T>>>,
    busy: Cell<bool>,
    done: Cell<bool>,
    has_faulted: Cell<bool>,
}

impl<T: 'static> FastImmediateObserver<T> {
    pub(crate) fn new(observer: Rc<dyn IObserver<T>>) -> Rc<Self> {
        Rc::new(Self {
            observer: RefCell::new(Some(observer)),
            queue: RefCell::new(VecDeque::new()),
            busy: Cell::new(false),
            done: Cell::new(false),
            has_faulted: Cell::new(false),
        })
    }

    fn drain(&self) {
        if self.busy.replace(true) {
            return;
        }
        struct Idle<'a, T>(&'a FastImmediateObserver<T>);
        impl<T> Drop for Idle<'_, T> {
            fn drop(&mut self) {
                if std::thread::panicking() {
                    self.0.has_faulted.set(true);
                    self.0.queue.borrow_mut().clear();
                }
                self.0.busy.set(false);
            }
        }
        let _idle = Idle(self);
        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some(next) = next else { break };
            let observer = self.observer.borrow().clone();
            let Some(observer) = observer else {
                self.queue.borrow_mut().clear();
                break;
            };
            next.accept(&*observer);
        }
    }
}

impl<T: 'static> IObserver<T> for FastImmediateObserver<T> {
    fn on_next(&self, value: T) {
        if !self.done.get() && !self.has_faulted.get() {
            self.queue.borrow_mut().push_back(Notification::OnNext(value));
        }
    }

    fn on_error(&self, error: ObservableError) {
        if !self.done.replace(true) && !self.has_faulted.get() {
            self.queue.borrow_mut().push_back(Notification::OnError(error));
        }
    }

    fn on_completed(&self) {
        if !self.done.replace(true) && !self.has_faulted.get() {
            self.queue.borrow_mut().push_back(Notification::OnCompleted);
        }
    }
}

impl<T: 'static> IDisposable for FastImmediateObserver<T> {
    fn dispose(&self) {
        self.done.set(true);
        self.observer.borrow_mut().take();
        if !self.busy.get() {
            self.queue.borrow_mut().clear();
        }
    }
}

impl<T: 'static> IScheduledObserver<T> for FastImmediateObserver<T> {
    fn ensure_active_n(&self, _n: usize) {
        self.drain();
    }
}
