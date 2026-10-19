//! `SynchronizationContextScheduler` (`Concurrency/SynchronizationContextScheduler.cs`), and the
//! posting of work that tells the context when it starts and ends (`PostWithStartComplete` of
//! `Internal/SynchronizationContextExtensions.cs`).
//!
//! Deviation (DEVIATIONS.md, Reactive): the work of a scheduler is not `Send`, so the scheduler
//! posts it with [`SynchronizationContext::post_local`]: it schedules on the context of its own
//! thread (the context that is current there, or one that runs its work on that thread), where
//! upstream may schedule from any thread. Delayed work waits on the default scheduler of the thread
//! (`DefaultScheduler.Instance`, [`SchedulerDefaults::default_scheduler`]) and is posted when it is
//! due, as upstream does with the default scheduler of the thread pool.

use super::local_scheduler::SystemStopwatchProvider;
use super::{IScheduler, IStopwatchProvider, ScheduledAction, Scheduler, SchedulerDefaults};
use crate::disposables::{ICancelable, SingleAssignmentDisposable};
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::IDisposable;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Tells the context that an operation ends when it is dropped, also while a panic unwinds (the
/// `finally` of `PostWithStartComplete`).
struct OperationCompleted<'a> {
    context: &'a Arc<dyn SynchronizationContext>,
}

impl Drop for OperationCompleted<'_> {
    fn drop(&mut self) {
        self.context.operation_completed();
    }
}

/// Posts `action` to `context` from its own thread, telling the context that an operation starts
/// now and ends when the action has run (`PostWithStartComplete`).
pub(crate) fn post_with_start_complete(context: &Arc<dyn SynchronizationContext>, action: impl FnOnce() + 'static) {
    context.operation_started();
    let target = context.clone();
    context.post_local(Box::new(move || {
        let _completed = OperationCompleted { context: &target };
        action();
    }));
}

/// Schedules work on a [`SynchronizationContext`] (`SynchronizationContextScheduler`): each work
/// is posted to the context, which runs it, unless the scheduler does not always post and the
/// context is the current one of the thread, which runs it at once.
pub struct SynchronizationContextScheduler {
    this: Weak<SynchronizationContextScheduler>,
    context: Arc<dyn SynchronizationContext>,
    always_post: bool,
}

impl SynchronizationContextScheduler {
    /// A scheduler that posts all its work to `context`
    /// (`SynchronizationContextScheduler(SynchronizationContext)`).
    pub fn new(context: Arc<dyn SynchronizationContext>) -> Rc<Self> {
        Self::with_always_post(context, true)
    }

    /// A scheduler of `context` that runs its work at once when `always_post` is false and
    /// `context` is the current context of the thread
    /// (`SynchronizationContextScheduler(SynchronizationContext, bool alwaysPost)`).
    pub fn with_always_post(context: Arc<dyn SynchronizationContext>, always_post: bool) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), context, always_post })
    }

    /// The context the scheduler posts to.
    pub fn context(&self) -> &Arc<dyn SynchronizationContext> {
        &self.context
    }

    fn this(&self) -> Rc<dyn IScheduler> {
        self.this.upgrade().expect("the scheduler is alive while it schedules")
    }
}

impl IScheduler for SynchronizationContextScheduler {
    fn now(&self) -> SystemTime {
        Scheduler::now()
    }

    /// `Schedule<TState>(TState, Func<IScheduler, TState, IDisposable>)`.
    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        let this = self.this();
        if !self.always_post && <dyn SynchronizationContext>::is_current(&self.context) {
            return action(&this);
        }

        let d = SingleAssignmentDisposable::new();
        let target = d.clone();
        post_with_start_complete(&self.context, move || {
            if !target.is_disposed() {
                target.set_disposable(Some(action(&this)));
            }
        });
        d
    }

    /// `Schedule<TState>(TState, TimeSpan, Func<IScheduler, TState, IDisposable>)`: work that is
    /// due now is scheduled at once, other work is scheduled when it is due.
    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        if due_time.is_zero() {
            return self.schedule(action);
        }
        let this = self.this();
        SchedulerDefaults::default_scheduler().schedule_after(due_time, Box::new(move |_: &Rc<dyn IScheduler>| this.schedule(action)))
    }

    /// `Schedule<TState>(TState, DateTimeOffset, Func<IScheduler, TState, IDisposable>)` of
    /// `LocalScheduler`: the time relative to now.
    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule_after(Scheduler::relative_to(self, due_time), action)
    }

    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        Some(&SystemStopwatchProvider)
    }
}
