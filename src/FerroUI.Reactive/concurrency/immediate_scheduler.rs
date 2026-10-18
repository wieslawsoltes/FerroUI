//! `ImmediateScheduler` (`ImmediateScheduler.cs`).

use super::local_scheduler::{sleep, SystemStopwatch, SystemStopwatchProvider};
use super::{AsyncLock, IScheduler, IStopwatchProvider, Scheduler, ScheduledAction};
use crate::disposables::SingleAssignmentDisposable;
use ferroui_system::IDisposable;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// Runs work at once on the current thread (`ImmediateScheduler`). Delayed work blocks the thread
/// for the delay. Work that the running work schedules on the scheduler it receives runs after it,
/// in order (the trampoline of `AsyncLockScheduler`), so recursion does not grow the stack.
pub struct ImmediateScheduler {
    _private: (),
}

impl ImmediateScheduler {
    /// The scheduler of the thread (`ImmediateScheduler.Instance`).
    pub fn instance() -> Rc<dyn IScheduler> {
        ferroui_system::ferro_thread_local! {
            static INSTANCE: Rc<dyn IScheduler> = Rc::new(ImmediateScheduler { _private: () });
        }
        INSTANCE.with(Rc::clone)
    }
}

impl IScheduler for ImmediateScheduler {
    fn now(&self) -> SystemTime {
        Scheduler::now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        let inner: Rc<dyn IScheduler> = AsyncLockScheduler::new();
        action(&inner)
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        sleep(due_time);
        let inner: Rc<dyn IScheduler> = AsyncLockScheduler::new();
        action(&inner)
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule_after(Scheduler::relative_to(self, due_time), action)
    }

    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        Some(&SystemStopwatchProvider)
    }
}

/// The scheduler the work of an immediate schedule receives (`AsyncLockScheduler`).
struct AsyncLockScheduler {
    this: Weak<AsyncLockScheduler>,
    lock: AsyncLock,
}

impl AsyncLockScheduler {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), lock: AsyncLock::new() })
    }

    fn this(&self) -> Rc<dyn IScheduler> {
        self.this.upgrade().expect("the scheduler is alive while its work runs")
    }
}

impl IScheduler for AsyncLockScheduler {
    fn now(&self) -> SystemTime {
        Scheduler::now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        let slot = SingleAssignmentDisposable::new();
        let this = self.this();
        let target = slot.clone();
        self.lock.wait(move || {
            if !crate::disposables::ICancelable::is_disposed(&*target) {
                target.set_disposable(Some(action(&this)));
            }
        });
        slot
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        if due_time.is_zero() {
            return self.schedule(action);
        }
        let timer = SystemStopwatch::start();
        let slot = SingleAssignmentDisposable::new();
        let this = self.this();
        let target = slot.clone();
        self.lock.wait(move || {
            if !crate::disposables::ICancelable::is_disposed(&*target) {
                use super::IStopwatch;
                sleep(due_time.saturating_sub(timer.elapsed()));
                if !crate::disposables::ICancelable::is_disposed(&*target) {
                    target.set_disposable(Some(action(&this)));
                }
            }
        });
        slot
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule_after(Scheduler::relative_to(self, due_time), action)
    }

    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        Some(&SystemStopwatchProvider)
    }
}
