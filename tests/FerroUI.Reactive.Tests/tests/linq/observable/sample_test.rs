//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SampleTest.cs` of Rx.NET.
//! Adapted: `TimeSpan.FromTicks` is `ticks_to_duration`. Upstream's `PeriodicTestScheduler` (of
//! `TimerTest.cs`) derives from the test scheduler; here it is a scheduler of this file that hands
//! the work to a test scheduler and offers periodic scheduling of its own, recording each run of a
//! timer (`TimerRun`). `Sample(IObservable<TSample>)` is `sample_by`.

use ferroui_reactive::concurrency::{IScheduler, ISchedulerPeriodic, IStopwatchProvider, ScheduledAction, SchedulerExt};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{IDisposable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// A run of a periodic timer: when it started, the times it ticked, and when it was stopped
/// (`TimerRun`; equal runs have the same start, stop and ticks).
#[derive(Clone, Debug, PartialEq)]
struct TimerRun {
    started: i64,
    stopped: i64,
    ticks: Vec<i64>,
}

impl TimerRun {
    /// `new TimerRun(started, stopped) { ticks }`.
    fn new(started: i64, stopped: i64, ticks: &[i64]) -> Self {
        Self { started, stopped, ticks: ticks.to_vec() }
    }
}

/// A test scheduler with periodic scheduling of its own (`PeriodicTestScheduler`).
struct PeriodicTestScheduler {
    test: TestScheduler,
    inner: Rc<dyn IScheduler>,
    timers: RefCell<Vec<Rc<RefCell<TimerRun>>>>,
}

impl PeriodicTestScheduler {
    fn new(test: &TestScheduler) -> Rc<Self> {
        Rc::new(Self { test: test.clone(), inner: test.scheduler(), timers: RefCell::new(Vec::new()) })
    }

    /// The runs of the timers (`Timers`).
    fn timers(&self) -> Vec<TimerRun> {
        self.timers.borrow().iter().map(|run| run.borrow().clone()).collect()
    }
}

impl IScheduler for PeriodicTestScheduler {
    fn now(&self) -> SystemTime {
        self.inner.now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.inner.schedule(action)
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.inner.schedule_after(due_time, action)
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.inner.schedule_at(due_time, action)
    }

    fn as_periodic(&self) -> Option<&dyn ISchedulerPeriodic> {
        Some(self)
    }

    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        self.inner.as_stopwatch_provider()
    }
}

impl ISchedulerPeriodic for PeriodicTestScheduler {
    fn schedule_periodic(&self, period: Duration, action: Box<dyn FnMut()>) -> Rc<dyn IDisposable> {
        let run = Rc::new(RefCell::new(TimerRun { started: self.test.clock(), stopped: 0, ticks: Vec::new() }));
        self.timers.borrow_mut().push(run.clone());

        let action = RefCell::new(action);
        let (clock, ticks) = (self.test.clone(), run.clone());
        let d = self.inner.schedule_recursive_after(period, move |recurse| {
            ticks.borrow_mut().ticks.push(clock.clock());
            (action.borrow_mut())();
            recurse(period);
        });

        let clock = self.test.clone();
        let stop = Disposable::create(move || {
            let now = clock.clock();
            run.borrow_mut().stopped = now;
        });
        CompositeDisposable::from_disposables(vec![stop, d])
    }
}

#[test]
fn sample_regular() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(230, 3),
        RT::on_next(260, 4),
        RT::on_next(300, 5),
        RT::on_next(350, 6),
        RT::on_next(380, 7),
        RT::on_completed(390),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_next(300, 5), // CHECK: boundary of sampling
        RT::on_next(350, 6),
        RT::on_next(400, 7), // Sample in last bucket
        RT::on_completed::<i32>(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));
}

#[test]
fn sample_periodic_regular() {
    let scheduler = TestScheduler::new();
    let periodic = PeriodicTestScheduler::new(&scheduler);

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(230, 3),
        RT::on_next(260, 4),
        RT::on_next(300, 5),
        RT::on_next(350, 6),
        RT::on_next(380, 7),
        RT::on_completed(390),
    ]);

    let (source, s): (_, Rc<dyn IScheduler>) = (xs.as_observable(), periodic.clone());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_next(300, 5), // CHECK: boundary of sampling
        RT::on_next(350, 6),
        RT::on_next(400, 7), // Sample in last bucket
        RT::on_completed::<i32>(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));

    assert_eq!(periodic.timers(), vec![TimerRun::new(200, 400, &[250, 300, 350, 400])]);
}

#[test]
fn sample_error_in_flight() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(230, 3),
        RT::on_next(260, 4),
        RT::on_next(300, 5),
        RT::on_next(310, 6),
        RT::on_error(330, ex.clone()),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_next(300, 5), // CHECK: boundary of sampling
        RT::on_error::<i32>(330, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 330));
}

#[test]
fn sample_periodic_error_in_flight() {
    let scheduler = TestScheduler::new();
    let periodic = PeriodicTestScheduler::new(&scheduler);

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(230, 3),
        RT::on_next(260, 4),
        RT::on_next(300, 5),
        RT::on_next(310, 6),
        RT::on_error(330, ex.clone()),
    ]);

    let (source, s): (_, Rc<dyn IScheduler>) = (xs.as_observable(), periodic.clone());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_next(300, 5), // CHECK: boundary of sampling
        RT::on_error::<i32>(330, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 330));

    assert_eq!(periodic.timers(), vec![TimerRun::new(200, 330, &[250, 300])]);
}

#[test]
fn sample_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn sample_periodic_empty() {
    let scheduler = TestScheduler::new();
    let periodic = PeriodicTestScheduler::new(&scheduler);

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);

    let (source, s): (_, Rc<dyn IScheduler>) = (xs.as_observable(), periodic.clone());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_eq!(periodic.timers(), vec![TimerRun::new(200, 300, &[250, 300])]);
}

#[test]
fn sample_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(300, ex.clone())]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(res.messages(), RT::on_error::<i32>(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn sample_periodic_error() {
    let scheduler = TestScheduler::new();
    let periodic = PeriodicTestScheduler::new(&scheduler);

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(300, ex.clone())]);

    let (source, s): (_, Rc<dyn IScheduler>) = (xs.as_observable(), periodic.clone());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(res.messages(), RT::on_error::<i32>(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_eq!(periodic.timers(), vec![TimerRun::new(200, 300, &[250])]);
}

#[test]
fn sample_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn sample_periodic_never() {
    let scheduler = TestScheduler::new();
    let periodic = PeriodicTestScheduler::new(&scheduler);

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (source, s): (_, Rc<dyn IScheduler>) = (xs.as_observable(), periodic.clone());
    let res = scheduler.start(move || source.sample_on(ticks_to_duration(50), s));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));

    assert_eq!(
        periodic.timers(),
        vec![TimerRun::new(200, 1000, &[250, 300, 350, 400, 450, 500, 550, 600, 650, 700, 750, 800, 850, 900, 950])]
    );
}

#[test]
fn sample_sampler_simple1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(300, 5),
        RT::on_next(310, 6),
        RT::on_completed(400),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(150, ""),
        RT::on_next(210, "bar"),
        RT::on_next(250, "foo"),
        RT::on_next(260, "qux"),
        RT::on_next(320, "baz"),
        RT::on_completed(500),
    ]);

    let (source, sampler) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.sample_by(sampler));

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_next(320, 6),
        RT::on_completed::<i32>(500), // on sampling boundaries only
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn sample_sampler_simple2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(300, 5),
        RT::on_next(310, 6),
        RT::on_next(360, 7),
        RT::on_completed(400),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(150, ""),
        RT::on_next(210, "bar"),
        RT::on_next(250, "foo"),
        RT::on_next(260, "qux"),
        RT::on_next(320, "baz"),
        RT::on_completed(500),
    ]);

    let (source, sampler) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.sample_by(sampler));

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_next(320, 6),
        RT::on_next(500, 7),
        RT::on_completed::<i32>(500), // on sampling boundaries only
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn sample_sampler_simple3() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_completed(300),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(150, ""),
        RT::on_next(210, "bar"),
        RT::on_next(250, "foo"),
        RT::on_next(260, "qux"),
        RT::on_next(320, "baz"),
        RT::on_completed(500),
    ]);

    let (source, sampler) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.sample_by(sampler));

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_next(320, 4),
        RT::on_completed::<i32>(320), // on sampling boundaries only
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 320));
}

#[test]
fn sample_sampler_completes_first() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_completed(600),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(150, ""),
        RT::on_next(210, "bar"),
        RT::on_next(250, "foo"),
        RT::on_next(260, "qux"),
        RT::on_next(320, "baz"),
        RT::on_completed(500),
    ]);

    let (source, sampler) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.sample_by(sampler));

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_next(320, 4),
        RT::on_completed::<i32>(600), // on sampling boundaries only
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn sample_sampler_source_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(300, 5),
        RT::on_next(310, 6),
        RT::on_error(320, ex.clone()),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(150, ""),
        RT::on_next(210, "bar"),
        RT::on_next(250, "foo"),
        RT::on_next(260, "qux"),
        RT::on_next(330, "baz"),
        RT::on_completed(400),
    ]);

    let (source, sampler) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.sample_by(sampler));

    assert_messages!(res.messages(), RT::on_next(250, 3), RT::on_error::<i32>(320, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 320));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 320));
}

#[test]
fn sample_sampler_sampler_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(300, 5),
        RT::on_next(310, 6),
        RT::on_completed(400),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(150, ""),
        RT::on_next(210, "bar"),
        RT::on_next(250, "foo"),
        RT::on_next(260, "qux"),
        RT::on_error(320, ex.clone()),
    ]);

    let (source, sampler) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.sample_by(sampler));

    assert_messages!(res.messages(), RT::on_next(250, 3), RT::on_error::<i32>(320, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 320));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 320));
}

// Not ported: Sample_ArgumentChecking - argument validation (and a negative TimeSpan, which a Duration cannot be)
// Not ported: Sample_DefaultScheduler_Periodic - needs operator ToEnumerable (the blocking operators, phase 3, later)
// Not ported: Sample_DefaultScheduler_PeriodicDisabled - needs operator ToEnumerable (the blocking operators, phase 3, later)
