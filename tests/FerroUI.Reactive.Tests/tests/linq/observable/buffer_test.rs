//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/BufferTest.cs` of Rx.NET.
//! Adapted: a buffer is a `Vec<i32>`, compared whole where upstream compares it with
//! `SequenceEqual`; the overloads are `buffer` (count), `buffer_with_skip` (count and skip),
//! `buffer_time_on` (time), `buffer_time_shift_on` (time and shift), `buffer_time_count_on` (time or
//! count), `buffer_closing` and `try_buffer_closing` (closing selector), `buffer_boundaries`,
//! `buffer_openings` (openings and a closing selector); `PeriodicTestScheduler` and `TimerRun` of
//! `TimerTest.cs` are local types.

use ferroui_reactive::concurrency::{IScheduler, ISchedulerPeriodic, IStopwatchProvider, ScheduledAction, SchedulerExt};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{IDisposable, IObservable, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{
    assert_messages, assert_subscriptions, duration_to_ticks, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// `string.Join(",", x.Select(xx => xx.ToString()).ToArray())`.
fn join(buffer: Vec<i32>) -> String {
    buffer.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn s(value: &str) -> String {
    value.to_owned()
}

/// A run of a periodic timer of [`PeriodicTestScheduler`]: when it started, when it stopped, and the
/// times it ticked (`TimerRun`).
#[derive(Clone, PartialEq, Debug)]
struct TimerRun {
    started: i64,
    stopped: Option<i64>,
    ticks: Vec<i64>,
}

impl TimerRun {
    fn new(started: i64, stopped: i64, ticks: Vec<i64>) -> Self {
        Self { started, stopped: Some(stopped), ticks }
    }
}

/// A test scheduler with periodic scheduling of its own, which records its timers
/// (`PeriodicTestScheduler`).
struct PeriodicTestScheduler {
    inner: TestScheduler,
    timers: Rc<RefCell<Vec<TimerRun>>>,
}

impl PeriodicTestScheduler {
    fn new(inner: &TestScheduler) -> Rc<Self> {
        Rc::new(Self { inner: inner.clone(), timers: Rc::new(RefCell::new(Vec::new())) })
    }

    fn timers(&self) -> Vec<TimerRun> {
        self.timers.borrow().clone()
    }
}

impl IScheduler for PeriodicTestScheduler {
    fn now(&self) -> SystemTime {
        self.inner.scheduler().now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.inner.scheduler().schedule(action)
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.inner.scheduler().schedule_after(due_time, action)
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.inner.scheduler().schedule_at(due_time, action)
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
        let index = {
            let mut timers = self.timers.borrow_mut();
            timers.push(TimerRun { started: self.inner.clock(), stopped: None, ticks: Vec::new() });
            timers.len() - 1
        };

        let action = RefCell::new(action);
        let (timers, clock) = (self.timers.clone(), self.inner.clone());
        let d = self.inner.scheduler().schedule_recursive_after(period, move |recurse| {
            timers.borrow_mut()[index].ticks.push(clock.clock());

            (action.borrow_mut())();
            recurse(period);
        });

        let (timers, clock) = (self.timers.clone(), self.inner.clone());
        CompositeDisposable::from_disposables(vec![Disposable::create(move || timers.borrow_mut()[index].stopped = Some(clock.clock())), d])
    }
}

/// The source of the boundary and closing tests.
fn closings_source(scheduler: &TestScheduler, end: ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<i32>>) -> Rc<ferroui_reactive_testing::HotObservable<i32>> {
    scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(180, 2),
        RT::on_next(250, 3),
        RT::on_next(260, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(410, 7),
        RT::on_next(420, 8),
        RT::on_next(470, 9),
        RT::on_next(550, 10),
        end,
    ])
}

/// The source of the count and time tests.
fn timed_source(scheduler: &TestScheduler, end: ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<i32>>) -> Rc<ferroui_reactive_testing::HotObservable<i32>> {
    scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(210, 2),
        RT::on_next(240, 3),
        RT::on_next(280, 4),
        RT::on_next(320, 5),
        RT::on_next(350, 6),
        RT::on_next(380, 7),
        RT::on_next(420, 8),
        RT::on_next(470, 9),
        end,
    ])
}

/// The source of the time-or-count tests.
fn time_or_count_source(scheduler: &TestScheduler, end: ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<i32>>) -> Rc<ferroui_reactive_testing::HotObservable<i32>> {
    scheduler.create_hot_observable(vec![
        RT::on_next(205, 1),
        RT::on_next(210, 2),
        RT::on_next(240, 3),
        RT::on_next(280, 4),
        RT::on_next(320, 5),
        RT::on_next(350, 6),
        RT::on_next(370, 7),
        RT::on_next(420, 8),
        RT::on_next(470, 9),
        end,
    ])
}

#[test]
fn buffer_closings_basic() {
    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let window = Rc::new(Cell::new(1i64));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || {
        source.buffer_closing(move || {
            let w = window.get();
            window.set(w + 1);
            Observable::timer_on(ticks_to_duration(w * 100), sched.clone())
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(300, vec![3, 4]),
        RT::on_next(500, vec![5, 6, 7, 8, 9]),
        RT::on_next(590, vec![10]),
        RT::on_completed(590),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));
}

#[test]
fn buffer_closings_inner_subscriptions() {
    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let closings = vec![
        scheduler.create_hot_observable(vec![RT::on_next(300, true), RT::on_next(350, false), RT::on_completed(380)]),
        scheduler.create_hot_observable(vec![RT::on_next(400, true), RT::on_next(510, false), RT::on_next(620, false)]),
        scheduler.create_hot_observable(vec![RT::on_completed::<bool>(500)]),
        scheduler.create_hot_observable(vec![RT::on_next(600, true)]),
    ];

    let window = Rc::new(Cell::new(0));

    let (source, selected) = (xs.as_observable(), closings.clone());
    let res = scheduler.start(move || {
        source.buffer_closing(move || {
            let index = window.get();
            window.set(index + 1);
            selected[index].as_observable()
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(300, vec![3, 4]),
        RT::on_next(400, vec![5, 6]),
        RT::on_next(500, vec![7, 8, 9]),
        RT::on_next(590, vec![10]),
        RT::on_completed(590)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));

    assert_subscriptions!(closings[0].subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(closings[1].subscriptions(), RT::subscribe(300, 400));

    assert_subscriptions!(closings[2].subscriptions(), RT::subscribe(400, 500));

    assert_subscriptions!(closings[3].subscriptions(), RT::subscribe(500, 590));
}

#[test]
fn buffer_closings_empty() {
    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let window = Rc::new(Cell::new(1i64));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || {
        source.buffer_closing(move || {
            let w = window.get();
            window.set(w + 1);
            Observable::empty::<i32>().delay_on(ticks_to_duration(w * 100), sched.clone())
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(300, vec![3, 4]),
        RT::on_next(500, vec![5, 6, 7, 8, 9]),
        RT::on_next(590, vec![10]),
        RT::on_completed(590),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));
}

#[test]
fn buffer_closings_dispose() {
    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let window = Rc::new(Cell::new(1i64));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start_disposed(
        move || {
            source.buffer_closing(move || {
                let w = window.get();
                window.set(w + 1);
                Observable::timer_on(ticks_to_duration(w * 100), sched.clone())
            })
        },
        400,
    );

    assert_messages!(res.messages(), RT::on_next(300, vec![3, 4]));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn buffer_closings_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = closings_source(&scheduler, RT::on_error(590, ex.clone()));

    let window = Rc::new(Cell::new(1i64));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || {
        source.buffer_closing(move || {
            let w = window.get();
            window.set(w + 1);
            Observable::timer_on(ticks_to_duration(w * 100), sched.clone())
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(300, vec![3, 4]),
        RT::on_next(500, vec![5, 6, 7, 8, 9]),
        RT::on_error(590, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));
}

#[test]
fn buffer_closings_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = closings_source(&scheduler, RT::on_error(590, new_error("other")));

    let (source, error) = (xs.as_observable(), ex.clone());
    let res = scheduler.start(move || source.try_buffer_closing(move || -> Result<Rc<dyn IObservable<i32>>, ObservableError> { Err(error.clone()) }));

    assert_messages!(res.messages(), RT::on_error::<Vec<i32>>(200, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 200));
}

#[test]
fn buffer_closings_window_close_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = closings_source(&scheduler, RT::on_error(590, new_error("other")));

    let (source, error, sched) = (xs.as_observable(), ex.clone(), scheduler.scheduler());
    let res = scheduler.start(move || source.buffer_closing(move || Observable::throw_on::<i32>(error.clone(), sched.clone())));

    assert_messages!(res.messages(), RT::on_error::<Vec<i32>>(201, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 201));
}

#[test]
fn buffer_opening_closings_basic() {
    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let ys = scheduler.create_hot_observable(vec![RT::on_next(255, 50), RT::on_next(330, 100), RT::on_next(350, 50), RT::on_next(400, 90), RT::on_completed(900)]);

    let (source, openings, sched) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.buffer_openings(openings, move |x: &i32| Observable::timer_on(ticks_to_duration(i64::from(*x)), sched.clone())));

    assert_messages!(
        res.messages(),
        RT::on_next(305, vec![4]),
        RT::on_next(400, vec![]),
        RT::on_next(430, vec![6, 7, 8]),
        RT::on_next(490, vec![7, 8, 9]),
        RT::on_completed(900),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 900));
}

#[test]
fn buffer_boundaries_simple() {
    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(255, true),
        RT::on_next(330, true),
        RT::on_next(350, true),
        RT::on_next(400, true),
        RT::on_next(500, true),
        RT::on_completed(900),
    ]);

    let (source, boundaries) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.buffer_boundaries(boundaries));

    assert_messages!(
        res.messages(),
        RT::on_next(255, vec![3]),
        RT::on_next(330, vec![4, 5]),
        RT::on_next(350, vec![6]),
        RT::on_next(400, vec![]),
        RT::on_next(500, vec![7, 8, 9]),
        RT::on_next(590, vec![10]),
        RT::on_completed(590)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 590));
}

#[test]
fn buffer_boundaries_on_completed_boundaries() {
    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let ys = scheduler.create_hot_observable(vec![RT::on_next(255, true), RT::on_next(330, true), RT::on_next(350, true), RT::on_completed(400)]);

    let (source, boundaries) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.buffer_boundaries(boundaries));

    assert_messages!(
        res.messages(),
        RT::on_next(255, vec![3]),
        RT::on_next(330, vec![4, 5]),
        RT::on_next(350, vec![6]),
        RT::on_next(400, vec![]),
        RT::on_completed(400)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn buffer_boundaries_on_error_source() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(180, 2),
        RT::on_next(250, 3),
        RT::on_next(260, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(380, 7),
        RT::on_error(400, ex.clone()),
    ]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(255, true), RT::on_next(330, true), RT::on_next(350, true), RT::on_completed(500)]);

    let (source, boundaries) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.buffer_boundaries(boundaries));

    assert_messages!(
        res.messages(),
        RT::on_next(255, vec![3]),
        RT::on_next(330, vec![4, 5]),
        RT::on_next(350, vec![6]),
        RT::on_error(400, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn buffer_boundaries_on_error_boundaries() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let ys = scheduler.create_hot_observable(vec![RT::on_next(255, true), RT::on_next(330, true), RT::on_next(350, true), RT::on_error(400, ex.clone())]);

    let (source, boundaries) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.buffer_boundaries(boundaries));

    assert_messages!(
        res.messages(),
        RT::on_next(255, vec![3]),
        RT::on_next(330, vec![4, 5]),
        RT::on_next(350, vec![6]),
        RT::on_error(400, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 400));
}

/// The source of the `Buffer_Count_*` tests.
fn count_source(scheduler: &TestScheduler, end: ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<i32>>) -> Rc<ferroui_reactive_testing::HotObservable<i32>> {
    scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(230, 4), RT::on_next(240, 5), end])
}

#[test]
fn buffer_count_partial_window() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(250));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.buffer(5));

    assert_messages!(res.messages(), RT::on_next(250, vec![2, 3, 4, 5]), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn buffer_count_full_windows() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(250));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.buffer(2));

    assert_messages!(res.messages(), RT::on_next(220, vec![2, 3]), RT::on_next(240, vec![4, 5]), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn buffer_count_full_and_partial_windows() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(250));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.buffer(3));

    assert_messages!(res.messages(), RT::on_next(230, vec![2, 3, 4]), RT::on_next(250, vec![5]), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn buffer_count_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = count_source(&scheduler, RT::on_error(250, ex.clone()));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.buffer(5));

    assert_messages!(res.messages(), RT::on_error::<Vec<i32>>(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn buffer_count_skip_less() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(250));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.buffer_with_skip(3, 1));

    assert_messages!(
        res.messages(),
        RT::on_next(230, vec![2, 3, 4]),
        RT::on_next(240, vec![3, 4, 5]),
        RT::on_next(250, vec![4, 5]),
        RT::on_next(250, vec![5]),
        RT::on_completed(250)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn buffer_count_skip_more() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(250));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.buffer_with_skip(2, 3));

    assert_messages!(res.messages(), RT::on_next(220, vec![2, 3]), RT::on_next(250, vec![5]), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn buffer_with_count_basic() {
    let scheduler = TestScheduler::new();

    let xs = timed_source(&scheduler, RT::on_completed(600));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.buffer_with_skip(3, 2).select(join));

    assert_messages!(
        res.messages(),
        RT::on_next(280, s("2,3,4")),
        RT::on_next(350, s("4,5,6")),
        RT::on_next(420, s("6,7,8")),
        RT::on_next(600, s("8,9")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn buffer_with_count_disposed() {
    let scheduler = TestScheduler::new();

    let xs = timed_source(&scheduler, RT::on_completed(600));

    let source = xs.as_observable();
    let res = scheduler.start_disposed(move || source.buffer_with_skip(3, 2).select(join), 370);

    assert_messages!(res.messages(), RT::on_next(280, s("2,3,4")), RT::on_next(350, s("4,5,6")));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 370));
}

#[test]
fn buffer_with_count_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = timed_source(&scheduler, RT::on_error(600, ex.clone()));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.buffer_with_skip(3, 2).select(join));

    assert_messages!(
        res.messages(),
        RT::on_next(280, s("2,3,4")),
        RT::on_next(350, s("4,5,6")),
        RT::on_next(420, s("6,7,8")),
        RT::on_error(600, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn buffer_with_time_basic1() {
    let scheduler = TestScheduler::new();

    let xs = timed_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.buffer_time_shift_on(ticks_to_duration(100), ticks_to_duration(70), sched).select(join));

    assert_messages!(
        res.messages(),
        RT::on_next(300, s("2,3,4")),
        RT::on_next(370, s("4,5,6")),
        RT::on_next(440, s("6,7,8")),
        RT::on_next(510, s("8,9")),
        RT::on_next(580, s("")),
        RT::on_next(600, s("")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn buffer_with_time_basic2() {
    let scheduler = TestScheduler::new();

    let xs = timed_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.buffer_time_shift_on(ticks_to_duration(70), ticks_to_duration(100), sched).select(join));

    assert_messages!(
        res.messages(),
        RT::on_next(270, s("2,3")),
        RT::on_next(370, s("5,6")),
        RT::on_next(470, s("8,9")),
        RT::on_next(570, s("")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn buffer_with_time_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = timed_source(&scheduler, RT::on_error(600, ex.clone()));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.buffer_time_shift_on(ticks_to_duration(100), ticks_to_duration(70), sched).select(join));

    assert_messages!(
        res.messages(),
        RT::on_next(300, s("2,3,4")),
        RT::on_next(370, s("4,5,6")),
        RT::on_next(440, s("6,7,8")),
        RT::on_next(510, s("8,9")),
        RT::on_next(580, s("")),
        RT::on_error(600, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn buffer_with_time_disposed() {
    let scheduler = TestScheduler::new();

    let xs = timed_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start_disposed(move || source.buffer_time_shift_on(ticks_to_duration(100), ticks_to_duration(70), sched).select(join), 370);

    assert_messages!(res.messages(), RT::on_next(300, s("2,3,4")));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 370));
}

#[test]
fn buffer_with_time_basic_same() {
    let scheduler = TestScheduler::new();

    let xs = timed_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.buffer_time_on(ticks_to_duration(100), sched).select(join));

    assert_messages!(
        res.messages(),
        RT::on_next(300, s("2,3,4")),
        RT::on_next(400, s("5,6,7")),
        RT::on_next(500, s("8,9")),
        RT::on_next(600, s("")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn buffer_with_time_basic_same_periodic() {
    let scheduler = TestScheduler::new();
    let periodic = PeriodicTestScheduler::new(&scheduler);

    let xs = timed_source(&scheduler, RT::on_completed(600));

    let source = xs.as_observable();
    let sched: Rc<dyn IScheduler> = periodic.clone();
    let res = scheduler.start(move || source.buffer_time_on(ticks_to_duration(100), sched).select(join));

    assert_messages!(
        res.messages(),
        RT::on_next(300, s("2,3,4")),
        RT::on_next(400, s("5,6,7")),
        RT::on_next(500, s("8,9")),
        RT::on_next(600, s("")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_eq!(vec![TimerRun::new(200, 600, vec![300, 400, 500])], periodic.timers());
}

#[test]
fn buffer_with_time_basic_same_periodic_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();
    let periodic = PeriodicTestScheduler::new(&scheduler);

    let xs = timed_source(&scheduler, RT::on_error(480, ex.clone()));

    let source = xs.as_observable();
    let sched: Rc<dyn IScheduler> = periodic.clone();
    let res = scheduler.start(move || source.buffer_time_on(ticks_to_duration(100), sched).select(join));

    assert_messages!(res.messages(), RT::on_next(300, s("2,3,4")), RT::on_next(400, s("5,6,7")), RT::on_error(480, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 480));

    assert_eq!(vec![TimerRun::new(200, 480, vec![300, 400])], periodic.timers());
}

#[test]
fn buffer_with_time_or_count_basic() {
    let scheduler = TestScheduler::new();

    let xs = time_or_count_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.buffer_time_count_on(ticks_to_duration(70), 3, sched).select(join));

    assert_messages!(
        res.messages(),
        RT::on_next(240, s("1,2,3")),
        RT::on_next(310, s("4")),
        RT::on_next(370, s("5,6,7")),
        RT::on_next(440, s("8")),
        RT::on_next(510, s("9")),
        RT::on_next(580, s("")),
        RT::on_next(600, s("")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn buffer_with_time_or_count_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = time_or_count_source(&scheduler, RT::on_error(600, ex.clone()));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.buffer_time_count_on(ticks_to_duration(70), 3, sched).select(join));

    assert_messages!(
        res.messages(),
        RT::on_next(240, s("1,2,3")),
        RT::on_next(310, s("4")),
        RT::on_next(370, s("5,6,7")),
        RT::on_next(440, s("8")),
        RT::on_next(510, s("9")),
        RT::on_next(580, s("")),
        RT::on_error(600, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn buffer_with_time_or_count_disposed() {
    let scheduler = TestScheduler::new();

    let xs = time_or_count_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start_disposed(move || source.buffer_time_count_on(ticks_to_duration(70), 3, sched).select(join), 370);

    assert_messages!(res.messages(), RT::on_next(240, s("1,2,3")), RT::on_next(310, s("4")), RT::on_next(370, s("5,6,7")));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 370));
}

#[test]
fn buffer_with_time_tick_while_on_completed() {
    let scheduler = TestScheduler::new();

    let clock = scheduler.clone();
    let _d = Observable::return_(1)
        .buffer_time_shift_on(Duration::from_millis(1), Duration::from_millis(2), scheduler.scheduler())
        .subscribe_next(move |_v| {
            clock.advance_by(duration_to_ticks(Duration::from_millis(1)));
        });
}

// Not ported: Buffer_ArgumentChecking - argument validation
// Not ported: Buffer_Single_ArgumentChecking - argument validation
// Not ported: BufferWithCount_ArgumentChecking - argument validation
// Not ported: BufferWithCount_Default - blocking operator First (waived: one thread)
// Not ported: Buffer_Time_ArgumentChecking - argument validation
// Not ported: BufferWithTime_ArgumentChecking - argument validation
// Not ported: BufferWithTime_Default - blocking operator First (waived: one thread)
// Not ported: BufferWithTimeOrCount_ArgumentChecking - argument validation
// Not ported: BufferWithTimeOrCount_Default - real threads (DefaultScheduler); blocking operator First (waived: one thread)
