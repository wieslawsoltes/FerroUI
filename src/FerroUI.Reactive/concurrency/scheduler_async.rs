//! The asynchronous members of `Scheduler` (`Concurrency/Scheduler.Async.cs`): `ScheduleAsync`,
//! which runs an asynchronous body on a scheduler, and `Yield` and `Sleep`, the operations it awaits
//! to hand the rest of its work back to the scheduler (`SchedulerOperation`,
//! `SchedulerOperationAwaiter` of `Concurrency/SchedulerOperation.cs`). The public methods are
//! those of [`SchedulerExt`](super::SchedulerExt); this module holds their machinery
//! (docs/porting/reactive.md, section 5.8).
//!
//! Deviation (DEVIATIONS.md, Reactive): the body of `ScheduleAsync` is a closure returning a future
//! of `Result<(), ObservableError>` (upstream's `Task`) or of `Result<Rc<dyn IDisposable>,
//! ObservableError>` (`Task<IDisposable>`), which runs as a task of the thread
//! (`Task::spawn_local`). `SchedulerOperation.ConfigureAwait(true)`, which posts the continuation to
//! the `SynchronizationContext` of the awaiting code, is left out: the port has no synchronization
//! context, and the continuation always runs on the scheduler (upstream's default,
//! `ConfigureAwait(false)`). An awaited operation that the token cancels ends with a
//! `TaskCanceledError` where upstream's `GetResult` throws `OperationCanceledException`.

use super::{IScheduler, ScheduledAction, SchedulerExt};
use crate::disposables::{Disposable, SingleAssignmentDisposableValue};
use ferroui_system::threading::tasks::{canceled, Task};
use ferroui_system::threading::{CancellationToken, CancellationTokenSource, LocalCancellationTokenRegistration};
use ferroui_system::{IDisposable, ObservableError};
use std::cell::{Cell, RefCell};
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, SystemTime};

/// What the body of [`schedule_async`](super::SchedulerExt::schedule_async) ends with: nothing
/// (`Func<.., Task>`), or the disposable of the work it started, which disposing the schedule
/// disposes (`Func<.., Task<IDisposable>>`).
pub trait ScheduleAsyncResult: Clone + 'static {
    /// The disposable that disposing the schedule disposes.
    fn into_disposable(self) -> Rc<dyn IDisposable>;
}

impl ScheduleAsyncResult for () {
    /// `ContinueWith(_ => Disposable.Empty)` of `InvokeAsync` for the bodies of a `Task`.
    fn into_disposable(self) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

impl ScheduleAsyncResult for Rc<dyn IDisposable> {
    fn into_disposable(self) -> Rc<dyn IDisposable> {
        self
    }
}

/// The scheduler that the body of `ScheduleAsync` receives (`CancelableScheduler`): it schedules
/// on the scheduler running the body and carries the token of the schedule, which `yield_` and
/// `sleep` without a token use.
struct CancelableScheduler {
    scheduler: Rc<dyn IScheduler>,
    token: CancellationToken,
}

impl IScheduler for CancelableScheduler {
    fn now(&self) -> SystemTime {
        self.scheduler.now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.scheduler.schedule(action)
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.scheduler.schedule_after(due_time, action)
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.scheduler.schedule_at(due_time, action)
    }

    fn cancellation_token(&self) -> Option<CancellationToken> {
        Some(self.token.clone())
    }
}

/// The token of a scheduler (`GetCancellationToken`): that of the scheduler of a `ScheduleAsync`
/// body, else none.
pub(crate) fn get_cancellation_token(scheduler: &Rc<dyn IScheduler>) -> CancellationToken {
    scheduler.cancellation_token().unwrap_or_else(CancellationToken::none)
}

/// One run of a `ScheduleAsync` body (`AsyncInvocation<TState>`): disposing it cancels the token of
/// the body and disposes what the body ended with.
struct AsyncInvocation {
    cts: CancellationTokenSource,
    run: SingleAssignmentDisposableValue,
}

impl IDisposable for AsyncInvocation {
    fn dispose(&self) {
        self.cts.cancel();
        self.run.dispose();
    }
}

/// `InvokeAsync`, `AsyncInvocation<TState>.Run`: starts the body on `scheduler` (at once, as an
/// `async` method runs until its first await), with the scheduler and token of the invocation.
pub(crate) fn invoke_async<R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
    scheduler: &Rc<dyn IScheduler>,
    action: impl FnOnce(Rc<dyn IScheduler>, CancellationToken) -> Fut,
) -> Rc<dyn IDisposable> {
    let invocation = Rc::new(AsyncInvocation { cts: CancellationTokenSource::new(), run: SingleAssignmentDisposableValue::new() });
    if invocation.cts.is_cancellation_requested() {
        return Disposable::empty();
    }
    let token = invocation.cts.token();
    let cancelable: Rc<dyn IScheduler> = Rc::new(CancelableScheduler { scheduler: scheduler.clone(), token: token.clone() });
    let task = Task::spawn_local(action(cancelable, token));
    let target = invocation.clone();
    // `TaskContinuationOptions.ExecuteSynchronously | NotOnCanceled`. A faulted body has nothing to
    // dispose: upstream rethrows its error inside the continuation, where nothing observes it.
    task.continue_with(move |task| {
        if task.is_canceled() {
            return;
        }
        if let Ok(result) = task.result() {
            target.run.set_disposable(Some(result.into_disposable()));
        }
    });
    invocation
}

/// How an operation schedules its continuation: on the scheduler, now, after a time or at a time.
type ScheduleContinuation = Rc<dyn Fn(Box<dyn FnOnce()>) -> Rc<dyn IDisposable>>;

/// An operation that an asynchronous body awaits to run its continuation on a scheduler
/// (`SchedulerOperation`), made by `yield_` and `sleep` of
/// [`SchedulerExt`](super::SchedulerExt). Awaiting it (`.await`, through its
/// [`get_awaiter`](Self::get_awaiter)) schedules the rest of the body on the scheduler, which
/// resumes it there; the operation ends with a `TaskCanceledError` when its token is cancelled.
/// Nothing is scheduled until it is awaited.
#[derive(Clone)]
pub struct SchedulerOperation {
    schedule: ScheduleContinuation,
    token: CancellationToken,
}

impl SchedulerOperation {
    pub(crate) fn new(schedule: ScheduleContinuation, token: CancellationToken) -> Self {
        Self { schedule, token }
    }

    /// The awaiter of the operation (`GetAwaiter`), which `.await` uses; each awaiter schedules the
    /// continuation once.
    pub fn get_awaiter(&self) -> SchedulerOperationAwaiter {
        SchedulerOperationAwaiter::new(self.schedule.clone(), self.token.clone())
    }
}

impl IntoFuture for SchedulerOperation {
    type Output = Result<(), ObservableError>;
    type IntoFuture = SchedulerOperationAwaiter;

    fn into_future(self) -> Self::IntoFuture {
        self.get_awaiter()
    }
}

struct AwaiterInner {
    schedule: ScheduleContinuation,
    token: CancellationToken,
    registration: RefCell<Option<LocalCancellationTokenRegistration>>,
    /// Whether a continuation was given (`_continuation != null`).
    awaited: Cell<bool>,
    /// The continuation until it runs (the guard `ran` of upstream).
    continuation: RefCell<Option<Box<dyn FnOnce()>>>,
    /// The scheduled continuation (`_work`).
    work: RefCell<Option<Rc<dyn IDisposable>>>,
    /// For the future: whether the continuation has run, and the waker it wakes.
    resumed: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl AwaiterInner {
    /// The guarded continuation: it runs once, from the scheduler or from the cancellation.
    fn run_continuation(&self) {
        let continuation = self.continuation.borrow_mut().take();
        if let Some(continuation) = continuation {
            let registration = self.registration.borrow_mut().take();
            if let Some(registration) = registration {
                registration.dispose();
            }
            continuation();
            // The work has run or been disposed; dropping it ends its hold on this awaiter.
            drop(self.work.borrow_mut().take());
        }
    }

    /// `Cancel`: the scheduled continuation is disposed and the continuation runs now.
    fn cancel(&self) {
        let work = self.work.borrow().clone();
        if let Some(work) = work {
            work.dispose();
        }
        self.run_continuation();
    }
}

impl Drop for AwaiterInner {
    fn drop(&mut self) {
        if let Some(registration) = self.registration.get_mut().take() {
            registration.dispose();
        }
    }
}

/// The awaiter of a [`SchedulerOperation`] (`SchedulerOperationAwaiter`): the members the `await`
/// of C# calls, and the [`Future`] that `.await` polls.
#[derive(Clone)]
pub struct SchedulerOperationAwaiter {
    inner: Rc<AwaiterInner>,
}

impl SchedulerOperationAwaiter {
    fn new(schedule: ScheduleContinuation, token: CancellationToken) -> Self {
        let inner = Rc::new(AwaiterInner {
            schedule,
            token: token.clone(),
            registration: RefCell::new(None),
            awaited: Cell::new(false),
            continuation: RefCell::new(None),
            work: RefCell::new(None),
            resumed: Cell::new(false),
            waker: RefCell::new(None),
        });
        if token.can_be_canceled() {
            let target = Rc::downgrade(&inner);
            let registration = token.register_local(move || {
                if let Some(target) = target.upgrade() {
                    target.cancel();
                }
            });
            *inner.registration.borrow_mut() = Some(registration);
        }
        Self { inner }
    }

    /// Whether the operation has ended: false unless its token is cancelled (`IsCompleted`).
    pub fn is_completed(&self) -> bool {
        self.inner.token.is_cancellation_requested()
    }

    /// The outcome of the operation: a `TaskCanceledError` when its token is cancelled
    /// (`GetResult`, which throws `OperationCanceledException`).
    pub fn get_result(&self) -> Result<(), ObservableError> {
        if self.inner.token.is_cancellation_requested() {
            return canceled();
        }
        Ok(())
    }

    /// Schedules `continuation` on the scheduler of the operation (`OnCompleted`); the cancellation
    /// of the token runs it at once instead, disposing what was scheduled.
    ///
    /// # Panics
    /// When the awaiter was given a continuation already (`InvalidOperationException`).
    pub fn on_completed(&self, continuation: impl FnOnce() + 'static) {
        assert!(!self.inner.awaited.replace(true), "The scheduler operation has already been awaited.");
        *self.inner.continuation.borrow_mut() = Some(Box::new(continuation));
        let target = self.inner.clone();
        let work = (self.inner.schedule)(Box::new(move || target.run_continuation()));
        // The continuation may have run already, from inside the scheduling.
        if self.inner.continuation.borrow().is_some() {
            *self.inner.work.borrow_mut() = Some(work);
        }
    }
}

impl Future for SchedulerOperationAwaiter {
    type Output = Result<(), ObservableError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = &self.inner;
        if !inner.awaited.get() {
            // `await`: an operation that has ended gives its result without scheduling.
            if self.is_completed() {
                return Poll::Ready(self.get_result());
            }
            *inner.waker.borrow_mut() = Some(cx.waker().clone());
            let state: Weak<AwaiterInner> = Rc::downgrade(inner);
            self.on_completed(move || {
                if let Some(state) = state.upgrade() {
                    state.resumed.set(true);
                    let waker = state.waker.borrow_mut().take();
                    if let Some(waker) = waker {
                        waker.wake();
                    }
                }
            });
        }
        if inner.resumed.get() {
            return Poll::Ready(self.get_result());
        }
        let mut waker = inner.waker.borrow_mut();
        if !waker.as_ref().is_some_and(|waker| waker.will_wake(cx.waker())) {
            *waker = Some(cx.waker().clone());
        }
        Poll::Pending
    }
}

/// `Yield(IScheduler, CancellationToken)`.
pub(crate) fn yield_on(scheduler: &Rc<dyn IScheduler>, token: CancellationToken) -> SchedulerOperation {
    let scheduler = scheduler.clone();
    SchedulerOperation::new(Rc::new(move |continuation: Box<dyn FnOnce()>| scheduler.schedule_action(continuation)), token)
}

/// `Sleep(IScheduler, TimeSpan, CancellationToken)`.
pub(crate) fn sleep_on(scheduler: &Rc<dyn IScheduler>, due_time: Duration, token: CancellationToken) -> SchedulerOperation {
    let scheduler = scheduler.clone();
    SchedulerOperation::new(
        Rc::new(move |continuation: Box<dyn FnOnce()>| scheduler.schedule_action_after(due_time, continuation)),
        token,
    )
}

/// `Sleep(IScheduler, DateTimeOffset, CancellationToken)`.
pub(crate) fn sleep_at_on(scheduler: &Rc<dyn IScheduler>, due_time: SystemTime, token: CancellationToken) -> SchedulerOperation {
    let scheduler = scheduler.clone();
    SchedulerOperation::new(
        Rc::new(move |continuation: Box<dyn FnOnce()>| scheduler.schedule_action_at(due_time, continuation)),
        token,
    )
}
