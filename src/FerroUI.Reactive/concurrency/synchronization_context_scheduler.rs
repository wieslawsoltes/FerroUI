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
//!
//! The `SynchronizationContext` of `ferroui-system` is deprecated since 0.5.0 (step A5 of phase C4
//! of #423): the context is a schedule of `ferroui-system` ([`Schedule`]; a dispatcher's
//! `task_schedule(priority)`, or `task::current_schedule()`), to which the work is posted with
//! [`Schedule::schedule_local`], and which is current when it is the current schedule of the
//! thread. The forms that take a `SynchronizationContext` remain, deprecated, with their behaviour
//! (`OperationStarted` and `OperationCompleted` included). The same holds for `ContextDisposable`,
//! `ObserveOn`, `SubscribeOn` and `NotifyOn` of a context.
#![allow(deprecated)]

use super::local_scheduler::SystemStopwatchProvider;
use super::{IScheduler, IStopwatchProvider, ScheduledAction, Scheduler, SchedulerDefaults};
use crate::disposables::{ICancelable, SingleAssignmentDisposable};
use ferroui_system::task::{is_current_schedule, Schedule};
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::IDisposable;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Where the work of the forms of Rx that take a context is posted: a schedule of `ferroui-system`,
/// or a deprecated `SynchronizationContext`.
#[derive(Clone)]
pub(crate) enum PostTarget {
    Schedule(Arc<dyn Schedule>),
    Context(Arc<dyn SynchronizationContext>),
}

impl PostTarget {
    /// Whether the target is the current one of the thread (`SynchronizationContext.Current ==
    /// context`): the current schedule, or the current context.
    pub(crate) fn is_current(&self) -> bool {
        match self {
            Self::Schedule(schedule) => is_current_schedule(schedule),
            Self::Context(context) => <dyn SynchronizationContext>::is_current(context),
        }
    }

    /// Posts `work`, which is not `Send`, from the thread of the target (`Post`).
    pub(crate) fn post_local(&self, work: Box<dyn FnOnce()>) {
        match self {
            Self::Schedule(schedule) => schedule.schedule_local(work),
            Self::Context(context) => context.post_local(work),
        }
    }

    /// Tells a context that an operation starts (`OperationStarted`); a schedule has no such
    /// notion.
    pub(crate) fn operation_started(&self) {
        if let Self::Context(context) = self {
            context.operation_started();
        }
    }

    /// Tells a context that an operation has completed (`OperationCompleted`).
    pub(crate) fn operation_completed(&self) {
        if let Self::Context(context) = self {
            context.operation_completed();
        }
    }
}

/// Tells the context that an operation ends when it is dropped, also while a panic unwinds (the
/// `finally` of `PostWithStartComplete`).
struct OperationCompleted<'a> {
    target: &'a PostTarget,
}

impl Drop for OperationCompleted<'_> {
    fn drop(&mut self) {
        self.target.operation_completed();
    }
}

/// Posts `action` to `target` from its own thread, telling a context that an operation starts
/// now and ends when the action has run (`PostWithStartComplete`).
pub(crate) fn post_with_start_complete(target: &PostTarget, action: impl FnOnce() + 'static) {
    target.operation_started();
    let completed = target.clone();
    target.post_local(Box::new(move || {
        let _completed = OperationCompleted { target: &completed };
        action();
    }));
}

/// Schedules work on a context (`SynchronizationContextScheduler`), a [`Schedule`] of
/// `ferroui-system`: each work is posted to it with [`Schedule::schedule_local`], unless the
/// scheduler does not always post and the schedule is the current one of the thread
/// (`ferroui_system::task::current_schedule`), which runs it at once.
pub struct SynchronizationContextScheduler {
    this: Weak<SynchronizationContextScheduler>,
    target: PostTarget,
    always_post: bool,
}

impl SynchronizationContextScheduler {
    /// A scheduler that posts all its work to `schedule`
    /// (`SynchronizationContextScheduler(SynchronizationContext)`).
    pub fn with_schedule(schedule: Arc<dyn Schedule>) -> Rc<Self> {
        Self::with_schedule_and_always_post(schedule, true)
    }

    /// A scheduler of `schedule` that runs its work at once when `always_post` is false and
    /// `schedule` is the current schedule of the thread
    /// (`SynchronizationContextScheduler(SynchronizationContext, bool alwaysPost)`).
    pub fn with_schedule_and_always_post(schedule: Arc<dyn Schedule>, always_post: bool) -> Rc<Self> {
        Self::with_target(PostTarget::Schedule(schedule), always_post)
    }

    /// A scheduler that posts all its work to `context`
    /// (`SynchronizationContextScheduler(SynchronizationContext)`).
    #[deprecated(since = "0.5.0", note = "use `SynchronizationContextScheduler::with_schedule`")]
    pub fn new(context: Arc<dyn SynchronizationContext>) -> Rc<Self> {
        Self::with_always_post(context, true)
    }

    /// A scheduler of `context` that runs its work at once when `always_post` is false and
    /// `context` is the current context of the thread
    /// (`SynchronizationContextScheduler(SynchronizationContext, bool alwaysPost)`).
    #[deprecated(since = "0.5.0", note = "use `SynchronizationContextScheduler::with_schedule_and_always_post`")]
    pub fn with_always_post(context: Arc<dyn SynchronizationContext>, always_post: bool) -> Rc<Self> {
        Self::with_target(PostTarget::Context(context), always_post)
    }

    pub(crate) fn with_target(target: PostTarget, always_post: bool) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), target, always_post })
    }

    /// The schedule the scheduler posts to (its context), `None` for a scheduler of a deprecated
    /// context. Not named `schedule`, which is the member of [`IScheduler`].
    pub fn context_schedule(&self) -> Option<&Arc<dyn Schedule>> {
        match &self.target {
            PostTarget::Schedule(schedule) => Some(schedule),
            PostTarget::Context(_) => None,
        }
    }

    /// The context the scheduler posts to.
    ///
    /// # Panics
    /// When the scheduler was made with a schedule ([`with_schedule`](Self::with_schedule)).
    #[deprecated(since = "0.5.0", note = "use `SynchronizationContextScheduler::context_schedule`")]
    pub fn context(&self) -> &Arc<dyn SynchronizationContext> {
        match &self.target {
            PostTarget::Context(context) => context,
            PostTarget::Schedule(_) => panic!("the scheduler posts to a schedule, not to a SynchronizationContext"),
        }
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
        if !self.always_post && self.target.is_current() {
            return action(&this);
        }

        let d = SingleAssignmentDisposable::new();
        let target = d.clone();
        post_with_start_complete(&self.target, move || {
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
