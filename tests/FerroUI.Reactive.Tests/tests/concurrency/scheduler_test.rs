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

use ferroui_reactive::concurrency::{
    IScheduler, ISchedulerPeriodic, IStopwatch, IStopwatchProvider, ScheduledAction, Scheduler, SchedulerExt,
};
use ferroui_reactive::IDisposable;
use ferroui_reactive_testing::{duration_to_ticks, ticks_to_duration, TestScheduler};
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
// Not ported: DisableOptimizations1 - real threads (TaskPoolScheduler); Scheduler.DisableOptimizations does not exist
// Not ported: DisableOptimizations2 - real threads (TaskPoolScheduler); Scheduler.DisableOptimizations and ISchedulerLongRunning do not exist
// Not ported: DisableOptimizations3 - real threads (TaskPoolScheduler); Scheduler.DisableOptimizations and ISchedulerLongRunning do not exist
// Not ported: DisableOptimizations_UnknownService - Scheduler.DisableOptimizations and the lookup of a service by type (IServiceProvider.GetService) do not exist
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
// Not ported: SchedulerAsync_ScheduleAsync_Overloads1 - needs async/await (phase 4)
// Not ported: SchedulerAsync_ScheduleAsync_Overloads2 - needs async/await (phase 4)
// Not ported: SchedulerAsync_ScheduleAsync_Overloads3 - needs async/await (phase 4)
// Not ported: SchedulerAsync_ScheduleAsync_NoCancellation1 - needs async/await (phase 4)
// Not ported: SchedulerAsync_ScheduleAsync_NoCancellation2 - needs async/await (phase 4)
// Not ported: SchedulerAsync_Awaiters - needs async/await (phase 4)
// Not ported: SchedulerAsync_Yield_Cancel1 - needs async/await (phase 4)
// Not ported: SchedulerAsync_Yield_Cancel2 - needs async/await (phase 4)
// Not ported: SchedulerAsync_Sleep_Cancel - needs async/await (phase 4); real threads (Default scheduler, ManualResetEvent)
// Not ported: SchedulerAsync_ScheduleAsync_SyncCtx - needs async/await (phase 4)
