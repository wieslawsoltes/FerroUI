//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/TimerTest.cs` of Rx.NET.
//! Adapted: `Timer(TimeSpan, ...)` is `Observable::timer_on`, `Timer(DateTimeOffset, ...)`
//! `timer_at_on`, `Timer(TimeSpan, TimeSpan, ...)` `timer_periodic_on` and
//! `Timer(DateTimeOffset, TimeSpan, ...)` `timer_at_periodic_on`; `new DateTimeOffset(t,
//! TimeSpan.Zero)` is `ticks_to_time(t)`; an observer that throws is one that panics, and the
//! scheduler that runs it panics: `assert_throws`, or `catch_unwind` where the test checks the clock
//! after the exception (the panic carries the message `"ex"`, where upstream checks the identity of
//! the exception); `PeriodicTestScheduler`, a subclass of `TestScheduler` upstream, is a scheduler
//! here that runs its work on a `TestScheduler` (passing itself to the work, as the subclass is
//! passed upstream) and adds the periodic scheduling that records its `TimerRun`s.

use ferroui_reactive::concurrency::{IScheduler, ISchedulerPeriodic, IStopwatchProvider, ScheduledAction, SchedulerExt};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{IDisposable, IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_throws, ticks_to_duration, ticks_to_time, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

type Slot<T> = Rc<RefCell<Option<T>>>;

fn slot<T>() -> Slot<T> {
    Rc::new(RefCell::new(None))
}

fn get<T: Clone>(slot: &Slot<T>) -> T {
    slot.borrow().clone().expect("the variable has been assigned")
}

fn set<T>(slot: &Slot<T>, value: T) {
    *slot.borrow_mut() = Some(value);
}

#[test]
fn one_shot_timer_time_span_basic() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::timer_on(ticks_to_duration(300), s));

    assert_messages!(res.messages(), RT::on_next(500, 0i64), RT::on_completed::<i64>(500));
}

#[test]
fn one_shot_timer_time_span_zero() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::timer_on(ticks_to_duration(0), s));

    assert_messages!(res.messages(), RT::on_next(201, 0i64), RT::on_completed::<i64>(201));
}

#[test]
fn one_shot_timer_time_span_disposed() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::timer_on(ticks_to_duration(1000), s));

    assert_messages!(res.messages());
}

#[test]
fn one_shot_timer_time_span_observer_throws() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::timer_on(ticks_to_duration(1), scheduler1.scheduler());

    xs.subscribe_next(|_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler1.run());

    let scheduler2 = TestScheduler::new();

    let ys = Observable::timer_on(ticks_to_duration(1), scheduler2.scheduler());

    ys.subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));

    assert_throws(|| scheduler2.run());
}

#[test]
fn one_shot_timer_date_time_offset_basic() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::timer_at_on(ticks_to_time(500), s));

    assert_messages!(res.messages(), RT::on_next(500, 0i64), RT::on_completed::<i64>(500));
}

#[test]
fn one_shot_timer_date_time_offset_zero() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::timer_at_on(ticks_to_time(200), s));

    assert_messages!(res.messages(), RT::on_next(201, 0i64), RT::on_completed::<i64>(201));
}

#[test]
fn one_shot_timer_date_time_offset_past() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::timer_at_on(ticks_to_time(0), s));

    assert_messages!(res.messages(), RT::on_next(201, 0i64), RT::on_completed::<i64>(201));
}

#[test]
fn repeating_timer_date_time_offset_time_span_simple() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_with(move || Observable::timer_at_periodic_on(ticks_to_time(300), ticks_to_duration(100), s), 0, 200, 750);

    assert_messages!(
        res.messages(),
        RT::on_next(300, 0i64),
        RT::on_next(400, 1i64),
        RT::on_next(500, 2i64),
        RT::on_next(600, 3i64),
        RT::on_next(700, 4i64)
    );
}

#[test]
fn repeating_timer_time_span_time_span_simple() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_with(move || Observable::timer_periodic_on(ticks_to_duration(100), ticks_to_duration(100), s), 0, 200, 750);

    assert_messages!(
        res.messages(),
        RT::on_next(300, 0i64),
        RT::on_next(400, 1i64),
        RT::on_next(500, 2i64),
        RT::on_next(600, 3i64),
        RT::on_next(700, 4i64)
    );
}

#[test]
fn repeating_timer_periodic1() {
    let scheduler = PeriodicTestScheduler::new();

    let s: Rc<dyn IScheduler> = scheduler.clone();
    let res = scheduler.test.start_with(move || Observable::timer_periodic_on(ticks_to_duration(50), ticks_to_duration(100), s), 0, 200, 700);

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0i64),
        RT::on_next(350, 1i64),
        RT::on_next(450, 2i64),
        RT::on_next(550, 3i64),
        RT::on_next(650, 4i64)
    );

    assert_eq!(vec![TimerRun::stopped(250, 700, vec![350, 450, 550, 650])], scheduler.timers());
}

#[test]
fn repeating_timer_periodic2() {
    let scheduler = PeriodicTestScheduler::new();

    let s: Rc<dyn IScheduler> = scheduler.clone();
    let res = scheduler.test.start_with(move || Observable::timer_periodic_on(ticks_to_duration(100), ticks_to_duration(100), s), 0, 200, 750);

    assert_messages!(
        res.messages(),
        RT::on_next(300, 0i64),
        RT::on_next(400, 1i64),
        RT::on_next(500, 2i64),
        RT::on_next(600, 3i64),
        RT::on_next(700, 4i64)
    );

    assert_eq!(vec![TimerRun::stopped(200, 750, vec![300, 400, 500, 600, 700])], scheduler.timers());
}

/// The scenario of the slippage tests: the timer created at 100, subscribed at 200 with `on_next`
/// (which receives the scheduler and the times recorded so far), disposed at 1000.
fn run_slippage(
    scheduler: &TestScheduler,
    create: impl FnOnce(Rc<dyn IScheduler>) -> Rc<dyn IObservable<i64>> + 'static,
    on_next: impl Fn(&TestScheduler, i64) + 'static,
) -> (Rc<RefCell<Vec<i64>>>, std::thread::Result<()>) {
    let xs: Slot<Rc<dyn IObservable<i64>>> = slot();
    {
        let (s, xs) = (scheduler.scheduler(), xs.clone());
        scheduler.schedule_absolute_action(100, move || set(&xs, create(s)));
    }

    let times = Rc::new(RefCell::new(Vec::new()));

    let d: Slot<Rc<dyn IDisposable>> = slot();
    {
        let (s, xs, d, times) = (scheduler.clone(), xs.clone(), d.clone(), times.clone());
        scheduler.schedule_absolute_action(200, move || {
            let subscription = get(&xs).subscribe_next(move |x| {
                times.borrow_mut().push(s.clock());
                on_next(&s, x);
            });
            set(&d, subscription);
        });
    }
    {
        let d = d.clone();
        scheduler.schedule_absolute_action(1000, move || get(&d).dispose());
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scheduler.run()));
    (times, result)
}

/// The `onNext` of `RepeatingTimer_UsingStopwatch_Slippage1` and `_Slippage2`.
fn slippage_on_next(scheduler: &TestScheduler, x: i64) {
    if x == 0 {
        return;
    }

    if x < 2 {
        scheduler.sleep(50);
        return;
    }

    if x < 4 {
        scheduler.sleep(120);
        return;
    }

    if x < 6 {
        scheduler.sleep(50);
    }
}

#[test]
fn repeating_timer_using_stopwatch_slippage1() {
    let scheduler = TestScheduler::new();

    let (times, result) = run_slippage(&scheduler, |s| Observable::timer_periodic_on(Duration::ZERO, ticks_to_duration(100), s), slippage_on_next);
    assert!(result.is_ok());

    assert_eq!(
        vec![
            201, // 1 off because of initial scheduling jump (InvokeStart)
            301, 401, 522, // 2 off because of 401 + 120 + 1 scheduling tick
            643, // 3 off because of 522 + 120 + 1 scheduling tick
            701, 801, 901,
        ],
        *times.borrow()
    );
}

#[test]
fn repeating_timer_using_stopwatch_slippage2() {
    let scheduler = TestScheduler::new();

    let (times, result) =
        run_slippage(&scheduler, |s| Observable::timer_periodic_on(ticks_to_duration(100), ticks_to_duration(100), s), slippage_on_next);
    assert!(result.is_ok());

    assert_eq!(
        vec![
            300, 400, 500, 621, // 1 off because of recursive scheduling beyond the target time
            742, // 2 off because of 621 + 120 + 1 scheduling tick
            800, 900,
        ],
        *times.borrow()
    );
}

#[test]
fn repeating_timer_using_stopwatch_slippage3_catch_up_from_long_invoke_start() {
    let scheduler = TestScheduler::new();

    let (times, result) = run_slippage(
        &scheduler,
        |s| Observable::timer_periodic_on(Duration::ZERO, ticks_to_duration(100), s),
        |scheduler, x| {
            if x == 0 {
                scheduler.sleep(350);
            }
        },
    );
    assert!(result.is_ok());

    assert_eq!(
        vec![
            201, // 1 off because of initial scheduling jump (InvokeStart)
            551, // catching up after excessive delay of 350 (target was 300)
            552, // catching up after excessive delay of 350 (target was 400)
            553, // catching up after excessive delay of 350 (target was 500)
            601, // back in sync
            701, 801, 901,
        ],
        *times.borrow()
    );
}

#[test]
fn repeating_timer_using_stopwatch_slippage3_catch_up_from_long_invoke_start_throws_first() {
    let scheduler = TestScheduler::new();

    let (_times, result) = run_slippage(
        &scheduler,
        |s| Observable::timer_periodic_on(Duration::ZERO, ticks_to_duration(100), s),
        |_, x| {
            if x == 0 {
                std::panic::panic_any("ex");
            }
        },
    );

    if let Err(payload) = result {
        assert_eq!(201, scheduler.clock());
        assert_eq!(Some(&"ex"), payload.downcast_ref::<&str>());
    }
}

#[test]
fn repeating_timer_using_stopwatch_slippage3_catch_up_from_long_invoke_start_throws_beyond_first() {
    let scheduler = TestScheduler::new();

    let (times, result) = run_slippage(
        &scheduler,
        |s| Observable::timer_periodic_on(Duration::ZERO, ticks_to_duration(100), s),
        |scheduler, x| {
            if x == 0 {
                scheduler.sleep(350);
                return;
            }

            if x == 5 {
                std::panic::panic_any("ex");
            }
        },
    );

    if let Err(payload) = result {
        assert_eq!(701, scheduler.clock());
        assert_eq!(Some(&"ex"), payload.downcast_ref::<&str>());
    }

    assert_eq!(
        vec![
            201, // 1 off because of initial scheduling jump (InvokeStart)
            551, // catching up after excessive delay of 350 (target was 300)
            552, // catching up after excessive delay of 350 (target was 400)
            553, // catching up after excessive delay of 350 (target was 500)
            601, // back in sync
            701,
        ],
        *times.borrow()
    );
}

/// A run of the periodic scheduling of [`PeriodicTestScheduler`]: when it started, the times of
/// its ticks, and when it was stopped (`TimerRun`).
#[derive(Clone, PartialEq, Debug)]
struct TimerRun {
    started: i64,
    stopped: Option<i64>,
    ticks: Vec<i64>,
}

impl TimerRun {
    fn new(started: i64) -> Self {
        Self { started, stopped: None, ticks: Vec::new() }
    }

    fn stopped(started: i64, stopped: i64, ticks: Vec<i64>) -> Self {
        Self { started, stopped: Some(stopped), ticks }
    }
}

/// A test scheduler with periodic scheduling of its own, which records each run of it
/// (`PeriodicTestScheduler`).
struct PeriodicTestScheduler {
    this: Weak<PeriodicTestScheduler>,
    test: TestScheduler,
    timers: RefCell<Vec<Rc<RefCell<TimerRun>>>>,
}

impl PeriodicTestScheduler {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), test: TestScheduler::new(), timers: RefCell::new(Vec::new()) })
    }

    fn this(&self) -> Rc<dyn IScheduler> {
        self.this.upgrade().expect("the scheduler is alive while its work runs")
    }

    /// The work, run with this scheduler as the scheduler it runs on.
    fn wrap(&self, action: ScheduledAction) -> ScheduledAction {
        let this = self.this();
        Box::new(move |_| action(&this))
    }

    fn timers(&self) -> Vec<TimerRun> {
        self.timers.borrow().iter().map(|run| run.borrow().clone()).collect()
    }
}

impl IScheduler for PeriodicTestScheduler {
    fn now(&self) -> SystemTime {
        self.test.scheduler().now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.test.scheduler().schedule(self.wrap(action))
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.test.scheduler().schedule_after(due_time, self.wrap(action))
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.test.scheduler().schedule_at(due_time, self.wrap(action))
    }

    fn as_periodic(&self) -> Option<&dyn ISchedulerPeriodic> {
        Some(self)
    }

    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        Some(&*self.test)
    }
}

impl ISchedulerPeriodic for PeriodicTestScheduler {
    fn schedule_periodic(&self, period: Duration, action: Box<dyn FnMut()>) -> Rc<dyn IDisposable> {
        let run = Rc::new(RefCell::new(TimerRun::new(self.test.clock())));
        self.timers.borrow_mut().push(run.clone());

        let action = RefCell::new(action);
        let (ticks, clock) = (run.clone(), self.test.clone());
        let d = self.this().schedule_recursive_after(period, move |recurse| {
            ticks.borrow_mut().ticks.push(clock.clock());
            (action.borrow_mut())();
            recurse(period);
        });

        let (stop, clock) = (run, self.test.clone());
        let stopped = Disposable::create(move || stop.borrow_mut().stopped = Some(clock.clock()));
        CompositeDisposable::from_disposables(vec![stopped, d])
    }
}

// Not ported: OneShotTimer_TimeSpan_ArgumentChecking - argument validation (null schedulers, a negative period: a `Duration` cannot be negative)
// Not ported: OneShotTimer_TimeSpan_Zero_DefaultScheduler - real threads (ManualResetEvent, the default scheduler)
// Not ported: OneShotTimer_TimeSpan_Negative - a negative TimeSpan (a `Duration` cannot be negative; upstream normalizes it to zero, the case of OneShotTimer_TimeSpan_Zero)
// Not ported: OneShotTimer_TimeSpan_DefaultScheduler - real threads (the default scheduler) and needs operator ToEnumerable (phase 3, later)
// Not ported: OneShotTimer_DateTimeOffset_DefaultScheduler - real threads (the default scheduler) and needs operator ToEnumerable (phase 3, later)
// Not ported: OneShotTimer_TimeSpan_TimeSpan_DefaultScheduler - real threads (the default scheduler) and needs operators ToEnumerable and Take (phase 3, later)
// Not ported: OneShotTimer_DateTimeOffset_TimeSpan_DefaultScheduler - real threads (the default scheduler) and needs operators ToEnumerable and Take (phase 3, later)
// Not ported: RepeatingTimer_TimeSpan_Zero_DefaultScheduler - real threads (ManualResetEvent, the default scheduler) and needs operator TakeWhile (phase 3, later)
// Not ported: RepeatingTimer_NoStopwatch_Slippage1 - needs DisableOptimizations of a scheduler (not in the port; its periodic emulation without a stopwatch measures with the clock of the scheduler, not with upstream's best-effort recursive form)
// Not ported: RepeatingTimer_NoStopwatch_Slippage2 - needs DisableOptimizations of a scheduler (not in the port; as RepeatingTimer_NoStopwatch_Slippage1)
// Not ported: RepeatingTimer_Start_CatchUp - real threads (Thread.Sleep, ManualResetEvent, the default scheduler)
// Not ported: RepeatingTimer_Start_CatchUp_Throws - real threads (ThreadPoolScheduler, the Catch of a scheduler)
