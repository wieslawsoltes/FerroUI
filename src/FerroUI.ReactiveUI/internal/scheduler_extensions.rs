//! `SchedulerExtensions` (`ReactiveUI.Shared/Internal/SchedulerExtensions.cs`).

use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler, SchedulerExt};
use ferroui_reactive::disposables::Disposable;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Whether `scheduler` is the immediate scheduler of the thread (`ReferenceEquals(scheduler,
/// Scheduler.Immediate)`).
pub(crate) fn is_immediate(scheduler: &Rc<dyn IScheduler>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(scheduler), Rc::as_ptr(&ImmediateScheduler::instance()))
}

/// Runs work inline when the scheduler is the immediate one, and schedules it otherwise
/// (`SchedulerExtensions`).
#[allow(dead_code)]
pub(crate) trait SchedulerExtensions {
    /// Runs `action` with `state` at once on the immediate scheduler, else schedules it
    /// (`ScheduleOrInline<TState>(TState, Func<IScheduler, TState, IDisposable>)`).
    fn schedule_or_inline_with<S: 'static>(
        &self,
        state: S,
        action: impl FnOnce(&Rc<dyn IScheduler>, S) -> Rc<dyn IDisposable> + 'static,
    ) -> Rc<dyn IDisposable>;

    /// Runs `action` at once on the immediate scheduler, else schedules it
    /// (`ScheduleOrInline(Action)`).
    fn schedule_or_inline(&self, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable>;
}

impl SchedulerExtensions for Rc<dyn IScheduler> {
    fn schedule_or_inline_with<S: 'static>(
        &self,
        state: S,
        action: impl FnOnce(&Rc<dyn IScheduler>, S) -> Rc<dyn IDisposable> + 'static,
    ) -> Rc<dyn IDisposable> {
        if is_immediate(self) {
            return action(self, state);
        }
        self.schedule(Box::new(move |scheduler: &Rc<dyn IScheduler>| action(scheduler, state)))
    }

    fn schedule_or_inline(&self, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        if is_immediate(self) {
            action();
            return Disposable::empty();
        }
        self.schedule_action(action)
    }
}
