//! `IScheduler`, the services a scheduler may offer (`ISchedulerPeriodic`, `IStopwatchProvider`,
//! `IStopwatch`) and the extension methods of `Scheduler` (`Scheduler.cs`, `Scheduler.Simple.cs`,
//! `Scheduler.Recursive.cs`, `Scheduler.Services.cs`, `Scheduler.Services.Emulation.cs`,
//! `Scheduler.Async.cs`); the wrapper of `DisableOptimizations` is in
//! `disable_optimizations_scheduler.rs`, the machinery of the asynchronous members in
//! `scheduler_async.rs`.

use super::scheduler_async::{ScheduleAsyncResult, SchedulerOperation};
use crate::disposables::{CompositeDisposable, Disposable, SingleAssignmentDisposable};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, ObservableError};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// The work a scheduler runs: it receives the scheduler it runs on (which the work may schedule
/// more work on, now or later) and returns the disposable that cancels what it started
/// (`Func<IScheduler, TState, IDisposable>`; the state of upstream is what the closure captures).
pub type ScheduledAction = Box<dyn FnOnce(&Rc<dyn IScheduler>) -> Rc<dyn IDisposable>>;

/// Schedules units of work (`IScheduler`).
///
/// Deviation (DEVIATIONS.md, Reactive): absolute times are `SystemTime` where upstream has
/// `DateTimeOffset`, and relative times `Duration` where upstream has `TimeSpan`. A `TimeSpan` may
/// be negative upstream and is normalized to zero by every scheduler (`Scheduler.Normalize`); a
/// `Duration` cannot be negative.
pub trait IScheduler {
    /// The scheduler's notion of the current time.
    fn now(&self) -> SystemTime;

    /// Schedules work to run as soon as possible.
    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable>;

    /// Schedules work to run after `due_time`.
    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable>;

    /// Schedules work to run at `due_time`.
    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable>;

    /// The periodic scheduling of the scheduler, if it has its own (`GetService(typeof(ISchedulerPeriodic))`).
    fn as_periodic(&self) -> Option<&dyn ISchedulerPeriodic> {
        None
    }

    /// The stopwatch of the scheduler, if it has its own (`GetService(typeof(IStopwatchProvider))`).
    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        None
    }

    /// The cancellation token of the scheduler that [`SchedulerExt::schedule_async`] hands its
    /// body, which [`SchedulerExt::yield_`] and [`SchedulerExt::sleep`] without a token observe
    /// (`GetCancellationToken`: `scheduler is CancelableScheduler cs ? cs.Token : None`); none for
    /// any other scheduler.
    fn cancellation_token(&self) -> Option<CancellationToken> {
        None
    }
}

/// Schedules periodic work (`ISchedulerPeriodic`).
pub trait ISchedulerPeriodic {
    /// Runs `action` every `period`, passing the state it returned the previous time
    /// (`SchedulePeriodic<TState>(TState, TimeSpan, Func<TState, TState>)`; the closure keeps the
    /// state).
    fn schedule_periodic(&self, period: Duration, action: Box<dyn FnMut()>) -> Rc<dyn IDisposable>;
}

/// Measures elapsed time (`IStopwatch`).
pub trait IStopwatch {
    /// The time since the stopwatch was started.
    fn elapsed(&self) -> Duration;
}

/// Starts stopwatches (`IStopwatchProvider`).
pub trait IStopwatchProvider {
    fn start_stopwatch(&self) -> Box<dyn IStopwatch>;
}

/// The static members of `Scheduler`.
pub struct Scheduler;

impl Scheduler {
    /// The current time of the system (`Scheduler.Now`).
    pub fn now() -> SystemTime {
        SystemTime::now()
    }

    /// The scheduler that runs work at once on the current thread (`Scheduler.Immediate`).
    pub fn immediate() -> Rc<dyn IScheduler> {
        super::ImmediateScheduler::instance()
    }

    /// The scheduler that queues work on the current thread (`Scheduler.CurrentThread`).
    pub fn current_thread() -> Rc<dyn IScheduler> {
        super::CurrentThreadScheduler::instance()
    }

    /// The time of `due_time` relative to now on `scheduler`, zero when it is past.
    pub(crate) fn relative_to(scheduler: &dyn IScheduler, due_time: SystemTime) -> Duration {
        due_time.duration_since(scheduler.now()).unwrap_or(Duration::ZERO)
    }
}

/// The extension methods of `Scheduler` on a scheduler (`Scheduler.Simple.cs`,
/// `Scheduler.Recursive.cs`, `Scheduler.Services.cs`, `Scheduler.Async.cs`).
pub trait SchedulerExt {
    /// Schedules an action to run as soon as possible (`Schedule(Action)`).
    fn schedule_action(&self, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action to run after `due_time` (`Schedule(TimeSpan, Action)`).
    fn schedule_action_after(&self, due_time: Duration, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action to run at `due_time` (`Schedule(DateTimeOffset, Action)`).
    fn schedule_action_at(&self, due_time: SystemTime, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action that can schedule itself again (`Schedule(Action<Action>)`).
    fn schedule_recursive(&self, action: impl Fn(&dyn Fn()) + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action with a state that can schedule itself again with a new state
    /// (`Schedule<TState>(TState, Action<TState, Action<TState>>)`).
    fn schedule_recursive_with<S: 'static>(&self, state: S, action: impl Fn(S, &dyn Fn(S)) + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action after `due_time` that can schedule itself again after a time it chooses
    /// (`Schedule(TimeSpan, Action<Action<TimeSpan>>)`).
    fn schedule_recursive_after(&self, due_time: Duration, action: impl Fn(&dyn Fn(Duration)) + 'static) -> Rc<dyn IDisposable>;

    /// Schedules an action with a state after `due_time` that can schedule itself again with a new
    /// state after a time it chooses (`Schedule<TState>(TState, TimeSpan, Action<TState, Action<TState, TimeSpan>>)`).
    fn schedule_recursive_after_with<S: 'static>(
        &self,
        state: S,
        due_time: Duration,
        action: impl Fn(S, &dyn Fn(S, Duration)) + 'static,
    ) -> Rc<dyn IDisposable>;

    /// Schedules an action at `due_time` that can schedule itself again at a time it chooses
    /// (`Schedule(DateTimeOffset, Action<Action<DateTimeOffset>>)`).
    fn schedule_recursive_at(&self, due_time: SystemTime, action: impl Fn(&dyn Fn(SystemTime)) + 'static) -> Rc<dyn IDisposable>;

    /// Runs `action` every `period` (`SchedulePeriodic(TimeSpan, Action)`): through the periodic
    /// scheduling of the scheduler if it has its own, else emulated with recursive scheduling.
    fn schedule_periodic_action(&self, period: Duration, action: impl FnMut() + 'static) -> Rc<dyn IDisposable>;

    /// Starts a stopwatch (`StartStopwatch`): the scheduler's own if it has one, else one that
    /// measures with the scheduler's clock.
    fn start_stopwatch(&self) -> Box<dyn IStopwatch>;

    /// The scheduler without the services it offers (`DisableOptimizations(IScheduler)`): it
    /// schedules on this one, answers none of the services of
    /// [`SchedulerOptimization::ALL`](super::SchedulerOptimization::ALL), and
    /// hands the work it runs a scheduler without them too, so that operators use the emulated
    /// forms (periodic scheduling through recursive scheduling, a stopwatch on the clock).
    fn disable_optimizations(&self) -> Rc<dyn IScheduler>;

    /// The scheduler without the services `optimizations` (`DisableOptimizations(IScheduler,
    /// params Type[])`): the other services are those of this one.
    fn disable_optimizations_of(&self, optimizations: &[super::SchedulerOptimization]) -> Rc<dyn IScheduler>;

    /// Runs an asynchronous body on the scheduler, as soon as possible
    /// (`ScheduleAsync(Func<IScheduler, CancellationToken, Task>)`,
    /// `ScheduleAsync(Func<IScheduler, CancellationToken, Task<IDisposable>>)`).
    ///
    /// The body receives a scheduler and a token. It runs on the scheduler, as a task of the thread,
    /// until its first await; awaiting [`yield_`](Self::yield_) or [`sleep`](Self::sleep) of the
    /// scheduler it received resumes it there. Disposing the result cancels the token, which ends a
    /// pending `yield_` or `sleep` with a `TaskCanceledError`, and disposes the disposable the body
    /// ended with (its future's output: `()`, or an `Rc<dyn IDisposable>`).
    fn schedule_async<R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        action: impl FnOnce(Rc<dyn IScheduler>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable>;

    /// [`schedule_async`](Self::schedule_async) with a state handed to the body
    /// (`ScheduleAsync<TState>(TState, Func<IScheduler, TState, CancellationToken, Task>)`, and its
    /// `Task<IDisposable>` form).
    fn schedule_async_with<S: 'static, R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        state: S,
        action: impl FnOnce(Rc<dyn IScheduler>, S, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable>;

    /// [`schedule_async`](Self::schedule_async) after `due_time`
    /// (`ScheduleAsync(TimeSpan, Func<IScheduler, CancellationToken, Task>)`, and its
    /// `Task<IDisposable>` form).
    fn schedule_async_after<R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        due_time: Duration,
        action: impl FnOnce(Rc<dyn IScheduler>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable>;

    /// [`schedule_async_with`](Self::schedule_async_with) after `due_time`
    /// (`ScheduleAsync<TState>(TState, TimeSpan, Func<IScheduler, TState, CancellationToken, Task>)`,
    /// and its `Task<IDisposable>` form).
    fn schedule_async_after_with<S: 'static, R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        state: S,
        due_time: Duration,
        action: impl FnOnce(Rc<dyn IScheduler>, S, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable>;

    /// [`schedule_async`](Self::schedule_async) at `due_time`
    /// (`ScheduleAsync(DateTimeOffset, Func<IScheduler, CancellationToken, Task>)`, and its
    /// `Task<IDisposable>` form).
    fn schedule_async_at<R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        due_time: SystemTime,
        action: impl FnOnce(Rc<dyn IScheduler>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable>;

    /// [`schedule_async_with`](Self::schedule_async_with) at `due_time`
    /// (`ScheduleAsync<TState>(TState, DateTimeOffset, Func<IScheduler, TState, CancellationToken, Task>)`,
    /// and its `Task<IDisposable>` form).
    fn schedule_async_at_with<S: 'static, R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        state: S,
        due_time: SystemTime,
        action: impl FnOnce(Rc<dyn IScheduler>, S, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable>;

    /// An operation whose await hands the rest of the work to the scheduler, to run after the work
    /// queued before it (`Yield(IScheduler)`); on the scheduler of a
    /// [`schedule_async`](Self::schedule_async) body, the token of the body cancels it.
    fn yield_(&self) -> SchedulerOperation;

    /// [`yield_`](Self::yield_) that the cancellation of `token` ends with a `TaskCanceledError`
    /// (`Yield(IScheduler, CancellationToken)`).
    fn yield_with_token(&self, token: CancellationToken) -> SchedulerOperation;

    /// An operation whose await runs the rest of the work on the scheduler after `due_time`
    /// (`Sleep(IScheduler, TimeSpan)`); on the scheduler of a
    /// [`schedule_async`](Self::schedule_async) body, the token of the body cancels it.
    fn sleep(&self, due_time: Duration) -> SchedulerOperation;

    /// [`sleep`](Self::sleep) that the cancellation of `token` ends with a `TaskCanceledError`
    /// (`Sleep(IScheduler, TimeSpan, CancellationToken)`).
    fn sleep_with_token(&self, due_time: Duration, token: CancellationToken) -> SchedulerOperation;

    /// An operation whose await runs the rest of the work on the scheduler at `due_time`
    /// (`Sleep(IScheduler, DateTimeOffset)`); on the scheduler of a
    /// [`schedule_async`](Self::schedule_async) body, the token of the body cancels it.
    fn sleep_at(&self, due_time: SystemTime) -> SchedulerOperation;

    /// [`sleep_at`](Self::sleep_at) that the cancellation of `token` ends with a
    /// `TaskCanceledError` (`Sleep(IScheduler, DateTimeOffset, CancellationToken)`).
    fn sleep_at_with_token(&self, due_time: SystemTime, token: CancellationToken) -> SchedulerOperation;
}

impl SchedulerExt for Rc<dyn IScheduler> {
    fn schedule_action(&self, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        self.schedule(Box::new(move |_| {
            action();
            Disposable::empty()
        }))
    }

    fn schedule_action_after(&self, due_time: Duration, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        self.schedule_after(
            due_time,
            Box::new(move |_| {
                action();
                Disposable::empty()
            }),
        )
    }

    fn schedule_action_at(&self, due_time: SystemTime, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        self.schedule_at(
            due_time,
            Box::new(move |_| {
                action();
                Disposable::empty()
            }),
        )
    }

    fn schedule_recursive(&self, action: impl Fn(&dyn Fn()) + 'static) -> Rc<dyn IDisposable> {
        self.schedule_recursive_with((), move |(), recurse| action(&|| recurse(())))
    }

    fn schedule_recursive_with<S: 'static>(&self, state: S, action: impl Fn(S, &dyn Fn(S)) + 'static) -> Rc<dyn IDisposable> {
        let action: Rc<dyn Fn(S, &dyn Fn(S))> = Rc::new(action);
        self.schedule(Box::new(move |scheduler| {
            let invoker = Rc::new(RecursiveInvoker { scheduler: scheduler.clone(), group: CompositeDisposable::new() });
            invoker.clone().invoke_now(state, action);
            invoker
        }))
    }

    fn schedule_recursive_after(&self, due_time: Duration, action: impl Fn(&dyn Fn(Duration)) + 'static) -> Rc<dyn IDisposable> {
        self.schedule_recursive_after_with((), due_time, move |(), recurse| action(&|time| recurse((), time)))
    }

    fn schedule_recursive_after_with<S: 'static>(
        &self,
        state: S,
        due_time: Duration,
        action: impl Fn(S, &dyn Fn(S, Duration)) + 'static,
    ) -> Rc<dyn IDisposable> {
        let action: Rc<dyn Fn(S, &dyn Fn(S, Duration))> = Rc::new(action);
        self.schedule_after(
            due_time,
            Box::new(move |scheduler| {
                let invoker = Rc::new(RecursiveInvoker { scheduler: scheduler.clone(), group: CompositeDisposable::new() });
                invoker.clone().invoke_now_after(state, action);
                invoker
            }),
        )
    }

    fn schedule_recursive_at(&self, due_time: SystemTime, action: impl Fn(&dyn Fn(SystemTime)) + 'static) -> Rc<dyn IDisposable> {
        let action: Rc<dyn Fn(&dyn Fn(SystemTime))> = Rc::new(action);
        self.schedule_at(
            due_time,
            Box::new(move |scheduler| {
                let invoker = Rc::new(RecursiveInvoker { scheduler: scheduler.clone(), group: CompositeDisposable::new() });
                invoker.clone().invoke_now_at(action);
                invoker
            }),
        )
    }

    fn schedule_periodic_action(&self, period: Duration, action: impl FnMut() + 'static) -> Rc<dyn IDisposable> {
        if let Some(periodic) = self.as_periodic() {
            return periodic.schedule_periodic(period, Box::new(action));
        }
        if self.as_stopwatch_provider().is_some() {
            return schedule_periodic_emulated(self, period, Box::new(action));
        }
        schedule_periodic_recursive(self, period, Box::new(action))
    }

    fn start_stopwatch(&self) -> Box<dyn IStopwatch> {
        if let Some(provider) = self.as_stopwatch_provider() {
            return provider.start_stopwatch();
        }
        Box::new(EmulatedStopwatch { scheduler: self.clone(), start: self.now() })
    }

    fn disable_optimizations(&self) -> Rc<dyn IScheduler> {
        super::disable_optimizations_scheduler::DisableOptimizationsScheduler::create(self.clone(), &super::SchedulerOptimization::ALL)
    }

    fn disable_optimizations_of(&self, optimizations: &[super::SchedulerOptimization]) -> Rc<dyn IScheduler> {
        super::disable_optimizations_scheduler::DisableOptimizationsScheduler::create(self.clone(), optimizations)
    }

    fn schedule_async<R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        action: impl FnOnce(Rc<dyn IScheduler>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable> {
        self.schedule(Box::new(move |scheduler| super::scheduler_async::invoke_async(scheduler, action)))
    }

    fn schedule_async_with<S: 'static, R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        state: S,
        action: impl FnOnce(Rc<dyn IScheduler>, S, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable> {
        self.schedule(Box::new(move |scheduler| {
            super::scheduler_async::invoke_async(scheduler, move |scheduler, token| action(scheduler, state, token))
        }))
    }

    fn schedule_async_after<R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        due_time: Duration,
        action: impl FnOnce(Rc<dyn IScheduler>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable> {
        self.schedule_after(due_time, Box::new(move |scheduler| super::scheduler_async::invoke_async(scheduler, action)))
    }

    fn schedule_async_after_with<S: 'static, R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        state: S,
        due_time: Duration,
        action: impl FnOnce(Rc<dyn IScheduler>, S, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable> {
        self.schedule_after(
            due_time,
            Box::new(move |scheduler| {
                super::scheduler_async::invoke_async(scheduler, move |scheduler, token| action(scheduler, state, token))
            }),
        )
    }

    fn schedule_async_at<R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        due_time: SystemTime,
        action: impl FnOnce(Rc<dyn IScheduler>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable> {
        self.schedule_at(due_time, Box::new(move |scheduler| super::scheduler_async::invoke_async(scheduler, action)))
    }

    fn schedule_async_at_with<S: 'static, R: ScheduleAsyncResult, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        &self,
        state: S,
        due_time: SystemTime,
        action: impl FnOnce(Rc<dyn IScheduler>, S, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IDisposable> {
        self.schedule_at(
            due_time,
            Box::new(move |scheduler| {
                super::scheduler_async::invoke_async(scheduler, move |scheduler, token| action(scheduler, state, token))
            }),
        )
    }

    fn yield_(&self) -> SchedulerOperation {
        super::scheduler_async::yield_on(self, super::scheduler_async::get_cancellation_token(self))
    }

    fn yield_with_token(&self, token: CancellationToken) -> SchedulerOperation {
        super::scheduler_async::yield_on(self, token)
    }

    fn sleep(&self, due_time: Duration) -> SchedulerOperation {
        super::scheduler_async::sleep_on(self, due_time, super::scheduler_async::get_cancellation_token(self))
    }

    fn sleep_with_token(&self, due_time: Duration, token: CancellationToken) -> SchedulerOperation {
        super::scheduler_async::sleep_on(self, due_time, token)
    }

    fn sleep_at(&self, due_time: SystemTime) -> SchedulerOperation {
        super::scheduler_async::sleep_at_on(self, due_time, super::scheduler_async::get_cancellation_token(self))
    }

    fn sleep_at_with_token(&self, due_time: SystemTime, token: CancellationToken) -> SchedulerOperation {
        super::scheduler_async::sleep_at_on(self, due_time, token)
    }
}

/// The state of a recursive schedule (`InvokeRecBaseState` and its three forms): the group of the
/// pending recursions, each of which leaves the group when it runs.
struct RecursiveInvoker {
    scheduler: Rc<dyn IScheduler>,
    group: Rc<CompositeDisposable>,
}

impl RecursiveInvoker {
    fn invoke_now<S: 'static>(self: Rc<Self>, state: S, action: Rc<dyn Fn(S, &dyn Fn(S))>) {
        let this = self.clone();
        let recursion = action.clone();
        action(state, &move |next: S| this.clone().schedule_next(next, recursion.clone()));
    }

    fn schedule_next<S: 'static>(self: Rc<Self>, state: S, action: Rc<dyn Fn(S, &dyn Fn(S))>) {
        let slot = SingleAssignmentDisposable::new();
        self.group.add(slot.clone());
        let this = self.clone();
        let entry = slot.clone();
        let scheduled = self.scheduler.schedule(Box::new(move |_| {
            this.group.remove(&(entry as Rc<dyn IDisposable>));
            this.invoke_now(state, action);
            Disposable::empty()
        }));
        slot.set_disposable(Some(scheduled));
    }

    fn invoke_now_after<S: 'static>(self: Rc<Self>, state: S, action: Rc<dyn Fn(S, &dyn Fn(S, Duration))>) {
        let this = self.clone();
        let recursion = action.clone();
        action(state, &move |next: S, time: Duration| this.clone().schedule_next_after(next, time, recursion.clone()));
    }

    fn schedule_next_after<S: 'static>(self: Rc<Self>, state: S, time: Duration, action: Rc<dyn Fn(S, &dyn Fn(S, Duration))>) {
        let slot = SingleAssignmentDisposable::new();
        self.group.add(slot.clone());
        let this = self.clone();
        let entry = slot.clone();
        let scheduled = self.scheduler.schedule_after(
            time,
            Box::new(move |_| {
                this.group.remove(&(entry as Rc<dyn IDisposable>));
                this.invoke_now_after(state, action);
                Disposable::empty()
            }),
        );
        slot.set_disposable(Some(scheduled));
    }

    fn invoke_now_at(self: Rc<Self>, action: Rc<dyn Fn(&dyn Fn(SystemTime))>) {
        let this = self.clone();
        let recursion = action.clone();
        action(&move |time: SystemTime| this.clone().schedule_next_at(time, recursion.clone()));
    }

    fn schedule_next_at(self: Rc<Self>, time: SystemTime, action: Rc<dyn Fn(&dyn Fn(SystemTime))>) {
        let slot = SingleAssignmentDisposable::new();
        self.group.add(slot.clone());
        let this = self.clone();
        let entry = slot.clone();
        let scheduled = self.scheduler.schedule_at(
            time,
            Box::new(move |_| {
                this.group.remove(&(entry as Rc<dyn IDisposable>));
                this.invoke_now_at(action);
                Disposable::empty()
            }),
        );
        slot.set_disposable(Some(scheduled));
    }
}

impl IDisposable for RecursiveInvoker {
    fn dispose(&self) {
        self.group.dispose();
    }
}

/// A stopwatch that measures with the clock of a scheduler (`EmulatedStopwatch`).
struct EmulatedStopwatch {
    scheduler: Rc<dyn IScheduler>,
    start: SystemTime,
}

impl IStopwatch for EmulatedStopwatch {
    fn elapsed(&self) -> Duration {
        self.scheduler.now().duration_since(self.start).unwrap_or(Duration::ZERO)
    }
}

/// Periodic scheduling on a scheduler without its own (`SchedulePeriodic_` of
/// `Scheduler.Services.Emulation.cs`): each run is scheduled at the start time plus a multiple of
/// the period, so that the time the action takes does not drift the schedule; a run that is late
/// catches up without waiting.
fn schedule_periodic_emulated(scheduler: &Rc<dyn IScheduler>, period: Duration, action: Box<dyn FnMut()>) -> Rc<dyn IDisposable> {
    let action = Rc::new(RefCell::new(action));
    let stopwatch = scheduler.start_stopwatch();
    let next_due = Rc::new(Cell::new(period));
    let state = Rc::new(PeriodicState { stopwatch, next_due, period, action });
    scheduler.schedule_recursive_after(period, move |recurse| {
        (state.action.borrow_mut())();
        let due = state.next_due.get() + state.period;
        state.next_due.set(due);
        let elapsed = state.stopwatch.elapsed();
        recurse(due.saturating_sub(elapsed));
    })
}

/// The commands of [`schedule_periodic_recursive`] (`TICK`, `DispatchStart`, `DispatchEnd`).
#[derive(Clone, Copy)]
enum PeriodicCommand {
    Tick,
    DispatchStart,
    DispatchEnd,
}

/// Periodic scheduling on a scheduler without a stopwatch (`SchedulePeriodicRecursive<TState>`):
/// the ticks keep the period, each scheduling the next before the action runs; a tick that comes
/// while the action is pending is counted, and the end of the action dispatches it at once.
fn schedule_periodic_recursive(scheduler: &Rc<dyn IScheduler>, period: Duration, action: Box<dyn FnMut()>) -> Rc<dyn IDisposable> {
    let action = RefCell::new(action);
    let pending_tick_count = Cell::new(0usize);
    let cancel = SingleAssignmentDisposable::new();
    let on_panic = cancel.clone();
    cancel.set_disposable(Some(scheduler.schedule_recursive_after_with(PeriodicCommand::Tick, period, move |command, recurse| {
        let mut command = command;
        loop {
            match command {
                PeriodicCommand::Tick => {
                    recurse(PeriodicCommand::Tick, period);
                    pending_tick_count.set(pending_tick_count.get() + 1);
                    if pending_tick_count.get() == 1 {
                        command = PeriodicCommand::DispatchStart;
                        continue;
                    }
                }
                PeriodicCommand::DispatchStart => {
                    // A panic of the action ends the schedule (`_cancel.Dispose()` before the rethrow).
                    let guard = DisposeOnPanic(&*on_panic);
                    (action.borrow_mut())();
                    std::mem::forget(guard);
                    recurse(PeriodicCommand::DispatchEnd, Duration::ZERO);
                }
                PeriodicCommand::DispatchEnd => {
                    pending_tick_count.set(pending_tick_count.get() - 1);
                    if pending_tick_count.get() > 0 {
                        recurse(PeriodicCommand::DispatchStart, Duration::ZERO);
                    }
                }
            }
            break;
        }
    })));
    cancel
}

/// Disposes the schedule when it is dropped by a panic.
struct DisposeOnPanic<'a>(&'a SingleAssignmentDisposable);

impl Drop for DisposeOnPanic<'_> {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

struct PeriodicState {
    stopwatch: Box<dyn IStopwatch>,
    next_due: Rc<Cell<Duration>>,
    period: Duration,
    action: Rc<RefCell<Box<dyn FnMut()>>>,
}
