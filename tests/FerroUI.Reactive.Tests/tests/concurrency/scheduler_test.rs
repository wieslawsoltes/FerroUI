//! Ported from `Tests.System.Reactive/Tests/Concurrency/SchedulerTest.cs` of Rx.NET.
//! Adapted: the state of `ScheduleAction(state, ...)` is what the scheduled closure captures; the
//! `Check` callback of the test scheduler `MyScheduler` receives the due time only (the tests
//! ignore the callback and state upstream also passes); `Scheduler_Periodic1` and
//! `Scheduler_Periodic2` run on the thread pool upstream and wait for an event, and run here on the
//! test scheduler (the emulation of periodic scheduling through recursive scheduling and the
//! stopwatch of the virtual clock), advancing the virtual clock by one period until the event is
//! set; `Scheduler.AsPeriodic` and `Scheduler.AsStopwatchProvider` are the methods `as_periodic` and
//! `as_stopwatch_provider` of `IScheduler`, and the services of `MyDumbScheduler2` are optional
//! fields where upstream has a dictionary keyed by type; the checks of `Scheduler.AsLongRunning`
//! are left out of `DetectServices_*` (`ISchedulerLongRunning` does not exist).
//! `DisableOptimizations1` runs on the task pool upstream and waits for an event after each
//! scheduling, and runs here on the test scheduler, running it after each scheduling; its due time
//! of `DateTimeOffset.Now.AddMilliseconds(10)` is ten milliseconds after the scheduler's `now`.
//! `Scheduler.DisableOptimizations(params Type[])` is `disable_optimizations_of`, with the services
//! named by `SchedulerOptimization`.
//! The bodies of `ScheduleAsync` return a future (a `Task` is one): a `Task` is a `Task<()>`, and the
//! `Task<int>` the overload tests return as a plain `Task` is one too; `Yield` is `yield_`, the
//! overloads of `Sleep` are `sleep`, `sleep_at` and their `_with_token` forms, and
//! `OperationCanceledException` is a `TaskCanceledError` result of `get_result`; the test token
//! `TestContext.CancellationToken` is a token that is never cancelled. `SchedulerAsync_Sleep_Cancel`
//! sleeps on the default scheduler upstream and waits for an event, and sleeps here on the test
//! scheduler, which the cancellation does not need to run.

use ferroui_reactive::concurrency::{
    IScheduler, ISchedulerPeriodic, IStopwatch, IStopwatchProvider, ScheduledAction, Scheduler, SchedulerExt,
};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{IDisposable, IObserver, ObservableError};
use ferroui_reactive_testing::{assert_messages, assert_throws, duration_to_ticks, ticks_to_duration, ticks_to_time, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::tasks::{TaskCanceledError, TaskCompletionSource};
use ferroui_system::threading::{CancellationToken, CancellationTokenSource};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// A scheduler that runs work at once, checking the due time of delayed work and counting it
/// (`MyScheduler`).
struct MyScheduler {
    this: Weak<MyScheduler>,
    now: SystemTime,
    check: Option<Box<dyn Fn(Duration)>>,
    wait_cycles: Cell<i64>,
}

impl MyScheduler {
    fn new() -> Rc<Self> {
        Self::with_now(SystemTime::now(), None)
    }

    fn with_now(now: SystemTime, check: Option<Box<dyn Fn(Duration)>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), now, check, wait_cycles: Cell::new(0) })
    }

    fn this(&self) -> Rc<dyn IScheduler> {
        self.this.upgrade().expect("the scheduler is alive while it runs work")
    }
}

impl IScheduler for MyScheduler {
    fn now(&self) -> SystemTime {
        self.now
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        action(&self.this())
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        (self.check.as_ref().expect("Check is set"))(due_time);
        self.wait_cycles.set(self.wait_cycles.get() + duration_to_ticks(due_time));
        action(&self.this())
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule_after(due_time.duration_since(self.now).unwrap_or(Duration::ZERO), action)
    }
}

/// Whether `a` and `b` are the same object (`Assert.Equal` on references of upstream).
fn same_object<T: ?Sized, U: ?Sized>(a: &T, b: &U) -> bool {
    std::ptr::addr_eq(a as *const T, b as *const U)
}

#[test]
fn scheduler_schedule_non_recursive() {
    let ms: Rc<dyn IScheduler> = MyScheduler::new();
    let res = Rc::new(Cell::new(false));
    let r = res.clone();
    ms.schedule_recursive(move |_a| r.set(true));
    assert!(res.get());
}

#[test]
fn scheduler_schedule_recursive() {
    let ms: Rc<dyn IScheduler> = MyScheduler::new();
    let i = Rc::new(Cell::new(0));
    let ii = i.clone();
    ms.schedule_recursive(move |a| {
        ii.set(ii.get() + 1);
        if ii.get() < 10 {
            a();
        }
    });
    assert_eq!(10, i.get());
}

#[test]
fn scheduler_schedule_with_state() {
    let ms: Rc<dyn IScheduler> = MyScheduler::new();
    let res = Rc::new(Cell::new(false));
    let r = res.clone();
    let state = "state";
    ms.schedule_action(move || {
        assert_eq!("state", state);
        r.set(true);
    });
    assert!(res.get());
}

#[test]
fn scheduler_schedule_with_time_non_recursive() {
    let now = SystemTime::now();
    let ms = MyScheduler::with_now(now, Some(Box::new(|t: Duration| assert!(t == Duration::ZERO))));
    let s: Rc<dyn IScheduler> = ms.clone();
    let res = Rc::new(Cell::new(false));
    let r = res.clone();
    s.schedule_recursive_at(now, move |_a| r.set(true));
    assert!(res.get());
    assert!(ms.wait_cycles.get() == 0);
}

#[test]
fn scheduler_schedule_with_time_recursive() {
    let now = SystemTime::now();
    let i = Rc::new(Cell::new(0));
    let ms = MyScheduler::with_now(now, Some(Box::new(|t: Duration| assert!(t == Duration::ZERO))));
    let s: Rc<dyn IScheduler> = ms.clone();
    let ii = i.clone();
    s.schedule_recursive_at(now, move |a| {
        ii.set(ii.get() + 1);
        if ii.get() < 10 {
            a(now);
        }
    });
    assert!(ms.wait_cycles.get() == 0);
    assert_eq!(10, i.get());
}

#[test]
fn scheduler_schedule_with_time_span_non_recursive() {
    let now = SystemTime::now();
    let ms = MyScheduler::with_now(now, Some(Box::new(|t: Duration| assert!(t == Duration::ZERO))));
    let s: Rc<dyn IScheduler> = ms.clone();
    let res = Rc::new(Cell::new(false));
    let r = res.clone();
    s.schedule_recursive_after(Duration::ZERO, move |_a| r.set(true));
    assert!(res.get());
    assert!(ms.wait_cycles.get() == 0);
}

#[test]
fn scheduler_schedule_with_time_span_recursive() {
    let now = SystemTime::now();
    let ms = MyScheduler::with_now(now, Some(Box::new(|t: Duration| assert!(t < ticks_to_duration(10)))));
    let s: Rc<dyn IScheduler> = ms.clone();
    let i = Rc::new(Cell::new(0));
    let ii = i.clone();
    s.schedule_recursive_after(Duration::ZERO, move |a| {
        ii.set(ii.get() + 1);
        if ii.get() < 10 {
            a(ticks_to_duration(ii.get()));
        }
    });
    assert!(ms.wait_cycles.get() == (1..=9).sum::<i64>());
    assert_eq!(10, i.get());
}

#[test]
fn scheduler_state_threading() {
    let lst = Rc::new(RefCell::new(Vec::new()));
    let l = lst.clone();
    Scheduler::immediate().schedule_recursive_with(0, move |i: i32, a| {
        l.borrow_mut().push(i);
        if i < 9 {
            a(i + 1);
        }
    });

    assert_eq!(*lst.borrow(), (0..10).collect::<Vec<i32>>());
}

#[test]
fn scheduler_periodic1() {
    let scheduler = TestScheduler::new();
    let n = Rc::new(Cell::new(0));
    let e = Rc::new(Cell::new(false));

    let (nn, ee) = (n.clone(), e.clone());
    let d = scheduler.scheduler().schedule_periodic_action(Duration::from_millis(50), move || {
        let v = nn.get();
        nn.set(v + 1);
        if v == 10 {
            ee.set(true);
        }
    });

    // e.WaitOne(): advance the virtual clock one period at a time until the event is set.
    let period = duration_to_ticks(Duration::from_millis(50));
    let mut periods = 0;
    while !e.get() {
        assert!(periods < 100, "the periodic action stopped running");
        scheduler.advance_by(period);
        periods += 1;
    }
    d.dispose();

    // The event is set by the eleventh run, one period after another.
    assert_eq!(11 * period, scheduler.clock());
}

#[test]
fn scheduler_periodic2() {
    let scheduler = TestScheduler::new();
    let n = Rc::new(Cell::new(0));
    let e = Rc::new(Cell::new(false));

    let state = 42;
    let (nn, ee) = (n.clone(), e.clone());
    let d = scheduler.scheduler().schedule_periodic_action(Duration::from_millis(50), move || {
        let x = state;
        assert_eq!(42, x);

        let v = nn.get();
        nn.set(v + 1);
        if v == 10 {
            ee.set(true);
        }
    });

    // e.WaitOne(): advance the virtual clock one period at a time until the event is set.
    let period = duration_to_ticks(Duration::from_millis(50));
    let mut periods = 0;
    while !e.get() {
        assert!(periods < 100, "the periodic action stopped running");
        scheduler.advance_by(period);
        periods += 1;
    }
    d.dispose();

    // The event is set by the eleventh run, one period after another.
    assert_eq!(11 * period, scheduler.clock());
}

#[test]
fn disable_optimizations1() {
    let scheduler = TestScheduler::new();
    let s = scheduler.scheduler();
    assert!(s.as_stopwatch_provider().is_some());

    let t = s.disable_optimizations();

    let d = t.now().duration_since(s.now()).unwrap_or(Duration::ZERO);
    assert!(d < Duration::from_secs(1));

    let e1 = Rc::new(Cell::new(false));
    {
        let e1 = e1.clone();
        t.schedule(Box::new(move |_| {
            e1.set(true);
            Disposable::empty()
        }));
    }
    scheduler.run();
    assert!(e1.get());

    let e2 = Rc::new(Cell::new(false));
    {
        let e2 = e2.clone();
        t.schedule_after(
            Duration::from_millis(1),
            Box::new(move |_| {
                e2.set(true);
                Disposable::empty()
            }),
        );
    }
    scheduler.run();
    assert!(e2.get());

    let e3 = Rc::new(Cell::new(false));
    {
        let e3 = e3.clone();
        t.schedule_at(
            t.now() + Duration::from_millis(10),
            Box::new(move |_| {
                e3.set(true);
                Disposable::empty()
            }),
        );
    }
    scheduler.run();
    assert!(e3.get());
}

#[test]
fn disable_optimizations_unknown_service() {
    let s: Rc<dyn IScheduler> = MyScheduler::new();
    assert!(s.as_periodic().is_none());
    assert!(s.as_stopwatch_provider().is_none());

    let d = s.disable_optimizations();
    // `GetService(typeof(bool))`: a service that is not hidden is asked of the scheduler wrapped,
    // which offers none. The services of the port are the ones `DisableOptimizations` hides, so
    // the lookup of a service that is not hidden is that of a wrapper hiding none.
    assert!(d.as_periodic().is_none());
    assert!(d.as_stopwatch_provider().is_none());
    let none_hidden = s.disable_optimizations_of(&[]);
    assert!(none_hidden.as_periodic().is_none());
    assert!(none_hidden.as_stopwatch_provider().is_none());
}

/// A scheduler with no services, whose members are not called (`MyDumbScheduler1`).
struct MyDumbScheduler1;

impl IScheduler for MyDumbScheduler1 {
    fn now(&self) -> SystemTime {
        unimplemented!()
    }

    fn schedule(&self, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        unimplemented!()
    }

    fn schedule_after(&self, _due_time: Duration, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        unimplemented!()
    }

    fn schedule_at(&self, _due_time: SystemTime, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        unimplemented!()
    }
}

#[test]
fn detect_services_null_1() {
    let s: Rc<dyn IScheduler> = Rc::new(MyDumbScheduler1);
    assert!(s.as_periodic().is_none());
    assert!(s.as_stopwatch_provider().is_none());
}

/// A scheduler that offers the services it is given, whose other members are not called
/// (`MyDumbScheduler2`).
struct MyDumbScheduler2 {
    periodic: Option<Rc<MyPeriodic>>,
    stopwatch_provider: Option<Rc<MyStopwatchProvider>>,
}

impl IScheduler for MyDumbScheduler2 {
    fn now(&self) -> SystemTime {
        unimplemented!()
    }

    fn schedule(&self, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        unimplemented!()
    }

    fn schedule_after(&self, _due_time: Duration, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        unimplemented!()
    }

    fn schedule_at(&self, _due_time: SystemTime, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        unimplemented!()
    }

    fn as_periodic(&self) -> Option<&dyn ISchedulerPeriodic> {
        self.periodic.as_deref().map(|p| p as &dyn ISchedulerPeriodic)
    }

    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        self.stopwatch_provider.as_deref().map(|p| p as &dyn IStopwatchProvider)
    }
}

/// A stopwatch provider whose members are not called (`MyStopwatchProvider`).
struct MyStopwatchProvider;

impl IStopwatchProvider for MyStopwatchProvider {
    fn start_stopwatch(&self) -> Box<dyn IStopwatch> {
        unimplemented!()
    }
}

/// A periodic scheduler whose members are not called (`MyPeriodic`).
struct MyPeriodic;

impl ISchedulerPeriodic for MyPeriodic {
    fn schedule_periodic(&self, _period: Duration, _action: Box<dyn FnMut()>) -> Rc<dyn IDisposable> {
        unimplemented!()
    }
}

#[test]
fn detect_services_null_2() {
    let s: Rc<dyn IScheduler> = Rc::new(MyDumbScheduler2 { periodic: None, stopwatch_provider: None });
    assert!(s.as_periodic().is_none());
    assert!(s.as_stopwatch_provider().is_none());
}

#[test]
fn detect_services_found() {
    {
        let x = Rc::new(MyStopwatchProvider);

        let s: Rc<dyn IScheduler> = Rc::new(MyDumbScheduler2 { periodic: None, stopwatch_provider: Some(x.clone()) });

        let found = s.as_stopwatch_provider().expect("the stopwatch provider is found");
        assert!(same_object(found, &*x));
    }

    {
        let x = Rc::new(MyPeriodic);

        let s: Rc<dyn IScheduler> = Rc::new(MyDumbScheduler2 { periodic: Some(x.clone()), stopwatch_provider: None });

        let found = s.as_periodic().expect("the periodic scheduler is found");
        assert!(same_object(found, &*x));
    }
}

#[test]
fn scheduler_async_schedule_async_overloads1() {
    let tcs_i = TaskCompletionSource::<()>::new();
    let t = tcs_i.task();
    tcs_i.set_result(());

    let tcs_d = TaskCompletionSource::<Rc<dyn IDisposable>>::new();
    let d = tcs_d.task();
    tcs_d.set_result(Disposable::empty());

    let s = TestScheduler::new();
    let scheduler = s.scheduler();

    let o = s.create_observer::<i32>();

    let (o1, t1) = (o.clone(), t.clone());
    scheduler.schedule_async(move |_, _ct| {
        o1.on_next(42);
        t1
    });

    let (o2, d2) = (o.clone(), d.clone());
    scheduler.schedule_async(move |_, _ct| {
        o2.on_next(43);
        d2
    });

    let (o3, t3) = (o.clone(), t.clone());
    scheduler.schedule_async_with(44, move |_, x, _ct| {
        o3.on_next(x);
        t3
    });

    let (o4, d4) = (o.clone(), d.clone());
    scheduler.schedule_async_with(45, move |_, _x, _ct| {
        o4.on_next(45);
        d4
    });

    s.run();

    assert_messages!(o.messages(), RT::on_next(1, 42), RT::on_next(1, 43), RT::on_next(1, 44), RT::on_next(1, 45));
}

#[test]
fn scheduler_async_schedule_async_overloads2() {
    let tcs_i = TaskCompletionSource::<()>::new();
    let t = tcs_i.task();
    tcs_i.set_result(());

    let tcs_d = TaskCompletionSource::<Rc<dyn IDisposable>>::new();
    let d = tcs_d.task();
    tcs_d.set_result(Disposable::empty());

    let s = TestScheduler::new();
    let scheduler = s.scheduler();

    let o = s.create_observer::<i32>();

    let (o1, t1) = (o.clone(), t.clone());
    scheduler.schedule_async_after(ticks_to_duration(50), move |_, _ct| {
        o1.on_next(42);
        t1
    });

    let (o2, d2) = (o.clone(), d.clone());
    scheduler.schedule_async_after(ticks_to_duration(60), move |_, _ct| {
        o2.on_next(43);
        d2
    });

    let (o3, t3) = (o.clone(), t.clone());
    scheduler.schedule_async_after_with(44, ticks_to_duration(70), move |_, x, _ct| {
        o3.on_next(x);
        t3
    });

    let (o4, d4) = (o.clone(), d.clone());
    scheduler.schedule_async_after_with(45, ticks_to_duration(80), move |_, _x, _ct| {
        o4.on_next(45);
        d4
    });

    s.run();

    assert_messages!(o.messages(), RT::on_next(50, 42), RT::on_next(60, 43), RT::on_next(70, 44), RT::on_next(80, 45));
}

#[test]
fn scheduler_async_schedule_async_overloads3() {
    let tcs_i = TaskCompletionSource::<()>::new();
    let t = tcs_i.task();
    tcs_i.set_result(());

    let tcs_d = TaskCompletionSource::<Rc<dyn IDisposable>>::new();
    let d = tcs_d.task();
    tcs_d.set_result(Disposable::empty());

    let s = TestScheduler::new();
    let scheduler = s.scheduler();

    let o = s.create_observer::<i32>();

    let (o1, t1) = (o.clone(), t.clone());
    scheduler.schedule_async_at(ticks_to_time(50), move |_, _ct| {
        o1.on_next(42);
        t1
    });

    let (o2, d2) = (o.clone(), d.clone());
    scheduler.schedule_async_at(ticks_to_time(60), move |_, _ct| {
        o2.on_next(43);
        d2
    });

    let (o3, t3) = (o.clone(), t.clone());
    scheduler.schedule_async_at_with(44, ticks_to_time(70), move |_, x, _ct| {
        o3.on_next(x);
        t3
    });

    let (o4, d4) = (o.clone(), d.clone());
    scheduler.schedule_async_at_with(45, ticks_to_time(80), move |_, _x, _ct| {
        o4.on_next(45);
        d4
    });

    s.run();

    assert_messages!(o.messages(), RT::on_next(50, 42), RT::on_next(60, 43), RT::on_next(70, 44), RT::on_next(80, 45));
}

#[test]
fn scheduler_async_schedule_async_no_cancellation1() {
    let s = TestScheduler::new();

    let o = s.create_observer::<i32>();

    let o1 = o.clone();
    s.scheduler().schedule_async(move |scheduler, _| async move {
        o1.on_next(42);

        scheduler.yield_().await?;

        o1.on_next(43);

        scheduler.sleep_with_token(ticks_to_duration(10), CancellationToken::none()).await?;

        o1.on_next(44);

        scheduler.sleep_at_with_token(ticks_to_time(250), CancellationToken::none()).await?;

        o1.on_next(45);
        Ok::<(), ObservableError>(())
    });

    s.run();

    assert_messages!(o.messages(), RT::on_next(1, 42), RT::on_next(2, 43), RT::on_next(12, 44), RT::on_next(250, 45));
}

#[test]
fn scheduler_async_schedule_async_no_cancellation2() {
    let s = TestScheduler::new();

    let o = s.create_observer::<i32>();

    let o1 = o.clone();
    s.scheduler().schedule_async(move |scheduler, ct| async move {
        o1.on_next(42);

        scheduler.yield_with_token(ct.clone()).await?;

        o1.on_next(43);

        scheduler.sleep_with_token(ticks_to_duration(10), ct.clone()).await?;

        o1.on_next(44);

        scheduler.sleep_at_with_token(ticks_to_time(250), ct).await?;

        o1.on_next(45);
        Ok::<(), ObservableError>(())
    });

    s.run();

    assert_messages!(o.messages(), RT::on_next(1, 42), RT::on_next(2, 43), RT::on_next(12, 44), RT::on_next(250, 45));
}

#[test]
fn scheduler_async_awaiters() {
    let op = Scheduler::immediate().yield_();
    let aw = op.get_awaiter();

    // `OnCompleted(null)` throws `ArgumentNullException`: argument validation.

    aw.on_completed(|| {});

    assert_throws(|| aw.on_completed(|| {}));
}

#[test]
fn scheduler_async_yield_cancel1() {
    let cts = CancellationTokenSource::new();

    let op = Scheduler::immediate().yield_with_token(cts.token());
    let aw = op.get_awaiter();

    cts.cancel();

    assert!(aw.is_completed());

    assert!(aw.get_result().is_err_and(|e| TaskCanceledError::is_cancellation(&e)));
}

#[test]
fn scheduler_async_yield_cancel2() {
    let cts = CancellationTokenSource::new();

    let op = Scheduler::immediate().yield_with_token(cts.token());
    let aw = op.get_awaiter();

    assert!(!aw.is_completed());

    let (source, awaiter) = (cts.clone(), aw.clone());
    aw.on_completed(move || {
        // Upstream leaves `Assert.True(aw.IsCompleted)` commented out here.

        source.cancel();

        assert!(awaiter.get_result().is_err_and(|e| TaskCanceledError::is_cancellation(&e)));
    });
}

#[test]
fn scheduler_async_sleep_cancel() {
    let s = TestScheduler::new();
    let cts = CancellationTokenSource::new();

    let op = s.scheduler().sleep_with_token(Duration::from_secs(3600), cts.token());
    let aw = op.get_awaiter();

    assert!(!aw.is_completed());

    let e = Rc::new(Cell::new(false));

    let (awaiter, set) = (aw.clone(), e.clone());
    aw.on_completed(move || {
        assert!(awaiter.get_result().is_err_and(|e| TaskCanceledError::is_cancellation(&e)));

        set.set(true);
    });

    cts.cancel();

    assert!(e.get(), "the continuation ran");
}

// Not ported: Scheduler_ArgumentChecks - argument validation
// Not ported: Schedulers_ArgumentChecks - argument validation
// Not ported: Scheduler_Builtins - real threads (Default, ThreadPool, NewThread and TaskPool schedulers, ManualResetEvent)
// Not ported: Scheduler_LongRunning_ArgumentChecking - argument validation
// Not ported: Scheduler_Periodic_ArgumentChecking - argument validation
// Not ported: Scheduler_Stopwatch_Emulation - argument validation
// Not ported: Scheduler_LongRunning1 - real threads (TaskPoolScheduler); ISchedulerLongRunning (ScheduleLongRunning) does not exist
// Not ported: Scheduler_LongRunning2 - real threads (TaskPoolScheduler); ISchedulerLongRunning (ScheduleLongRunning) does not exist
// Not ported: Scheduler_Periodic_HostLifecycleManagement - real threads (ThreadPoolScheduler, AppDomain, host lifecycle notifications of the platform enlightenment provider)
// Not ported: DisableOptimizations_ArgumentChecking - argument validation
// Not ported: DisableOptimizations2 - real threads (TaskPoolScheduler); ISchedulerLongRunning does not exist (the only service it checks)
// Not ported: DisableOptimizations3 - real threads (TaskPoolScheduler); ISchedulerLongRunning does not exist (the service it hides, keeping ISchedulerPeriodic)
// Not ported: Catch_ArgumentChecking - argument validation
// Not ported: Catch_Builtin_Swallow_Shallow - real threads (Default scheduler, ManualResetEvent); Scheduler.Catch does not exist
// Not ported: Catch_Builtin_Swallow_Recursive - real threads (Default scheduler, ManualResetEvent); Scheduler.Catch does not exist
// Not ported: Catch_Custom_Unhandled - Scheduler.Catch does not exist (nor LocalScheduler as a base class)
// Not ported: Catch_Custom_Rethrow - Scheduler.Catch does not exist (nor LocalScheduler as a base class)
// Not ported: Catch_Custom_LongRunning_Caught - Scheduler.Catch and ISchedulerLongRunning do not exist
// Not ported: Catch_Custom_LongRunning_Rethrow - Scheduler.Catch and ISchedulerLongRunning do not exist
// Not ported: Catch_Custom_Periodic_Regular - real threads (Default scheduler, ManualResetEvent); Scheduler.Catch does not exist
// Not ported: Catch_Custom_Periodic_Uncaught1 - real threads (Default scheduler, ManualResetEvent); Scheduler.Catch does not exist
// Not ported: Catch_Custom_Periodic_Uncaught2 - real threads (Default scheduler, ManualResetEvent); Scheduler.Catch does not exist
// Not ported: Catch_Custom_Periodic_Caught - real threads (Default scheduler, ManualResetEvent); Scheduler.Catch does not exist
// Not ported: InvalidService_Null - the lookup of a service by type (IServiceProvider.GetService) and LocalScheduler as a base class do not exist; the services are the typed methods as_periodic and as_stopwatch_provider
// Not ported: SchedulerAsync_Yield_ArgumentChecking - argument validation
// Not ported: SchedulerAsync_Sleep_ArgumentChecking - argument validation
// Not ported: SchedulerAsync_ScheduleAsync_ArgumentChecking - argument validation
// Not ported: SchedulerAsync_ScheduleAsync_SyncCtx - SynchronizationContext and SchedulerOperation.ConfigureAwait(true): the port has no synchronization context, and the continuation always runs on the scheduler
