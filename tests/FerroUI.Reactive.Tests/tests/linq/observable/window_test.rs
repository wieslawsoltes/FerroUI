//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/WindowTest.cs` of Rx.NET.
//! Adapted: a window is a `Rc<dyn IObservable<i32>>`; the overloads are `window` (count),
//! `window_with_skip` (count and skip), `window_time_on` (time), `window_time_shift_on` (time and
//! shift), `window_time_count_on` (time or count), `window_closing` and `try_window_closing` (closing
//! selector), `window_boundaries`; `.Select((w, i) => ...).Merge()` is `select_with_index` and
//! `merge_inner`; the variables that scheduled actions assign are shared slots; `PeriodicTestScheduler`
//! and `TimerRun` of `TimerTest.cs` are local types.

use ferroui_reactive::concurrency::{IScheduler, ISchedulerPeriodic, IStopwatchProvider, ScheduledAction, SchedulerExt};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{IDisposable, IObservable, Notification, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{
    assert_messages, assert_subscriptions, new_error, ticks_to_duration, HotObservable, MockObserver, ReactiveTest as RT, Recorded, TestScheduler,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

type Window = Rc<dyn IObservable<i32>>;

/// A variable that scheduled actions assign and read (`var x = default(X);` captured by lambdas).
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

fn s(value: &str) -> String {
    value.to_owned()
}

/// `.Select((w, i) => w.Select(x => i.ToString() + " " + x.ToString())).Merge()`.
fn labeled(windows: Rc<dyn IObservable<Window>>) -> Rc<dyn IObservable<String>> {
    windows.select_with_index(|w: Window, i| w.select(move |x| format!("{i} {x}"))).merge_inner()
}

/// `.Select((ys, i) => ys.Select(y => i + " " + y).Concat(Observable.Return(i + " end"))).Merge()`.
fn labeled_with_end(windows: Rc<dyn IObservable<Window>>) -> Rc<dyn IObservable<String>> {
    windows.select_with_index(|ys: Window, i| ys.select(move |y| format!("{i} {y}")).concat(Observable::return_(format!("{i} end")))).merge_inner()
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

/// The source of the closing and boundary tests.
fn closings_source(scheduler: &TestScheduler, end: Recorded<Notification<i32>>) -> Rc<HotObservable<i32>> {
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

/// The source of the count tests and of the time tests with a shift.
fn count_source(scheduler: &TestScheduler, end: Recorded<Notification<i32>>) -> Rc<HotObservable<i32>> {
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

/// The source of the `Window_Time_*` tests.
fn time_source(scheduler: &TestScheduler, values: &[(i64, i32)], end: Recorded<Notification<i32>>) -> Rc<HotObservable<i32>> {
    let mut messages: Vec<Recorded<Notification<i32>>> = values.iter().map(|&(time, value)| RT::on_next(time, value)).collect();
    messages.push(end);
    scheduler.create_hot_observable(messages)
}

const TIME_VALUES: [(i64, i32); 10] = [(150, 1), (210, 2), (240, 3), (270, 4), (320, 5), (360, 6), (390, 7), (410, 8), (460, 9), (470, 10)];

/// The source of the `WindowWithTimeOrCount_*` tests.
fn time_or_count_source(scheduler: &TestScheduler, end: Recorded<Notification<i32>>) -> Rc<HotObservable<i32>> {
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
fn window_closings_inner_subscriptions() {
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
        labeled(source.window_closing(move || {
            let index = window.get();
            window.set(index + 1);
            selected[index].as_observable()
        }))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, s("0 3")),
        RT::on_next(260, s("0 4")),
        RT::on_next(310, s("1 5")),
        RT::on_next(340, s("1 6")),
        RT::on_next(410, s("2 7")),
        RT::on_next(420, s("2 8")),
        RT::on_next(470, s("2 9")),
        RT::on_next(550, s("3 10")),
        RT::on_completed(590)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));

    assert_subscriptions!(closings[0].subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(closings[1].subscriptions(), RT::subscribe(300, 400));

    assert_subscriptions!(closings[2].subscriptions(), RT::subscribe(400, 500));

    assert_subscriptions!(closings[3].subscriptions(), RT::subscribe(500, 590));
}

#[test]
fn window_closings_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = closings_source(&scheduler, RT::on_error(590, new_error("other")));

    let (source, error) = (xs.as_observable(), ex.clone());
    let res = scheduler.start(move || labeled(source.try_window_closing(move || -> Result<Rc<dyn IObservable<i32>>, ObservableError> { Err(error.clone()) })));

    assert_messages!(res.messages(), RT::on_error::<String>(200, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 200));
}

#[test]
fn window_closings_window_close_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = closings_source(&scheduler, RT::on_error(590, new_error("other")));

    let (source, error, sched) = (xs.as_observable(), ex.clone(), scheduler.scheduler());
    let res = scheduler.start(move || labeled(source.window_closing(move || Observable::throw_on::<i32>(error.clone(), sched.clone()))));

    assert_messages!(res.messages(), RT::on_error::<String>(201, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 201));
}

#[test]
fn window_boundaries_simple() {
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
    let res = scheduler.start(move || labeled(source.window_boundaries(boundaries)));

    assert_messages!(
        res.messages(),
        RT::on_next(250, s("0 3")),
        RT::on_next(260, s("1 4")),
        RT::on_next(310, s("1 5")),
        RT::on_next(340, s("2 6")),
        RT::on_next(410, s("4 7")),
        RT::on_next(420, s("4 8")),
        RT::on_next(470, s("4 9")),
        RT::on_next(550, s("5 10")),
        RT::on_completed(590)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 590));
}

#[test]
fn window_boundaries_on_completed_boundaries() {
    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let ys = scheduler.create_hot_observable(vec![RT::on_next(255, true), RT::on_next(330, true), RT::on_next(350, true), RT::on_completed(400)]);

    let (source, boundaries) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || labeled(source.window_boundaries(boundaries)));

    assert_messages!(
        res.messages(),
        RT::on_next(250, s("0 3")),
        RT::on_next(260, s("1 4")),
        RT::on_next(310, s("1 5")),
        RT::on_next(340, s("2 6")),
        RT::on_completed(400)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn window_boundaries_on_error_source() {
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
    let res = scheduler.start(move || labeled(source.window_boundaries(boundaries)));

    assert_messages!(
        res.messages(),
        RT::on_next(250, s("0 3")),
        RT::on_next(260, s("1 4")),
        RT::on_next(310, s("1 5")),
        RT::on_next(340, s("2 6")),
        RT::on_next(380, s("3 7")),
        RT::on_error(400, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn window_boundaries_on_error_boundaries() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = closings_source(&scheduler, RT::on_completed(590));

    let ys = scheduler.create_hot_observable(vec![RT::on_next(255, true), RT::on_next(330, true), RT::on_next(350, true), RT::on_error(400, ex.clone())]);

    let (source, boundaries) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || labeled(source.window_boundaries(boundaries)));

    assert_messages!(
        res.messages(),
        RT::on_next(250, s("0 3")),
        RT::on_next(260, s("1 4")),
        RT::on_next(310, s("1 5")),
        RT::on_next(340, s("2 6")),
        RT::on_error(400, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn window_with_count_basic() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(600));

    let source = xs.as_observable();
    let res = scheduler.start(move || labeled(source.window_with_skip(3, 2)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("0 4")),
        RT::on_next(280, s("1 4")),
        RT::on_next(320, s("1 5")),
        RT::on_next(350, s("1 6")),
        RT::on_next(350, s("2 6")),
        RT::on_next(380, s("2 7")),
        RT::on_next(420, s("2 8")),
        RT::on_next(420, s("3 8")),
        RT::on_next(470, s("3 9")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

/// The state of the `WindowWithCount_InnerTimings*` tests: the windows as they arrive, an observer
/// of each (subscribed one tick later), the subscriptions to them, and the times they arrived.
struct InnerTimings {
    res: Slot<Rc<dyn IObservable<Window>>>,
    outer_subscription: Slot<Rc<dyn IDisposable>>,
    inner_subscriptions: Rc<RefCell<Vec<Rc<dyn IDisposable>>>>,
    windows: Rc<RefCell<Vec<Window>>>,
    observers: Rc<RefCell<Vec<Rc<MockObserver<i32>>>>>,
    window_creation_times: Rc<RefCell<Vec<i64>>>,
}

/// Schedules the creation of `xs.Window(3, 2)` and the subscription of the observer that records
/// the windows, as `WindowWithCount_InnerTimings*` do.
fn inner_timings(scheduler: &TestScheduler, xs: &Rc<HotObservable<i32>>) -> InnerTimings {
    let state = InnerTimings {
        res: slot(),
        outer_subscription: slot(),
        inner_subscriptions: Rc::new(RefCell::new(Vec::new())),
        windows: Rc::new(RefCell::new(Vec::new())),
        observers: Rc::new(RefCell::new(Vec::new())),
        window_creation_times: Rc::new(RefCell::new(Vec::new())),
    };

    {
        let (res, source) = (state.res.clone(), xs.as_observable());
        scheduler.schedule_absolute_action(RT::CREATED, move || set(&res, source.window_with_skip(3, 2)));
    }

    {
        let res = state.res.clone();
        let outer_subscription = state.outer_subscription.clone();
        let inner_subscriptions = state.inner_subscriptions.clone();
        let windows = state.windows.clone();
        let observers = state.observers.clone();
        let window_creation_times = state.window_creation_times.clone();
        let sched = scheduler.clone();
        scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || {
            let subscription = get(&res).subscribe_next(move |window: Window| {
                window_creation_times.borrow_mut().push(sched.clock());

                let result = sched.create_observer::<i32>();
                windows.borrow_mut().push(window.clone());
                observers.borrow_mut().push(result.clone());
                let inner_subscriptions = inner_subscriptions.clone();
                sched.schedule_absolute_action(0, move || inner_subscriptions.borrow_mut().push(window.subscribe(result)));
            });
            set(&outer_subscription, subscription);
        });
    }

    state
}

#[test]
fn window_with_count_inner_timings() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(600));

    let state = inner_timings(&scheduler, &xs);

    scheduler.run();

    let observers = state.observers.borrow();
    assert_eq!(5, observers.len());

    assert_messages!(observers[0].messages(), RT::on_next(210, 2), RT::on_next(240, 3), RT::on_next(280, 4), RT::on_completed(280));

    assert_messages!(observers[1].messages(), RT::on_next(280, 4), RT::on_next(320, 5), RT::on_next(350, 6), RT::on_completed(350));

    assert_messages!(observers[2].messages(), RT::on_next(350, 6), RT::on_next(380, 7), RT::on_next(420, 8), RT::on_completed(420));

    assert_messages!(observers[3].messages(), RT::on_next(420, 8), RT::on_next(470, 9), RT::on_completed(600));

    assert_messages!(observers[4].messages(), RT::on_completed::<i32>(600));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn window_with_count_inner_timings_dispose_outer() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(600));

    let state = inner_timings(&scheduler, &xs);

    {
        let outer_subscription = state.outer_subscription.clone();
        scheduler.schedule_absolute_action(400, move || get(&outer_subscription).dispose());
    }

    scheduler.run();

    assert!(*state.window_creation_times.borrow().last().expect("a window") < 400);

    let observers = state.observers.borrow();
    assert_eq!(4, observers.len());

    assert_messages!(observers[0].messages(), RT::on_next(210, 2), RT::on_next(240, 3), RT::on_next(280, 4), RT::on_completed(280));

    assert_messages!(observers[1].messages(), RT::on_next(280, 4), RT::on_next(320, 5), RT::on_next(350, 6), RT::on_completed(350));

    assert_messages!(observers[2].messages(), RT::on_next(350, 6), RT::on_next(380, 7), RT::on_next(420, 8), RT::on_completed(420));

    assert_messages!(observers[3].messages(), RT::on_next(420, 8), RT::on_next(470, 9), RT::on_completed(600));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn window_with_count_inner_timings_dispose_outer_and_inners() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(600));

    let state = inner_timings(&scheduler, &xs);

    {
        let outer_subscription = state.outer_subscription.clone();
        let inner_subscriptions = state.inner_subscriptions.clone();
        scheduler.schedule_absolute_action(400, move || {
            get(&outer_subscription).dispose();
            let subscriptions = inner_subscriptions.borrow().clone();
            for d in subscriptions {
                d.dispose();
            }
        });
    }

    scheduler.run();

    assert!(*state.window_creation_times.borrow().last().expect("a window") < 400);

    let observers = state.observers.borrow();
    assert_eq!(4, observers.len());

    assert_messages!(observers[0].messages(), RT::on_next(210, 2), RT::on_next(240, 3), RT::on_next(280, 4), RT::on_completed(280));

    assert_messages!(observers[1].messages(), RT::on_next(280, 4), RT::on_next(320, 5), RT::on_next(350, 6), RT::on_completed(350));

    assert_messages!(observers[2].messages(), RT::on_next(350, 6), RT::on_next(380, 7));

    assert_messages!(observers[3].messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn window_with_count_disposed() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(600));

    let source = xs.as_observable();
    let res = scheduler.start_disposed(move || labeled(source.window_with_skip(3, 2)), 370);

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("0 4")),
        RT::on_next(280, s("1 4")),
        RT::on_next(320, s("1 5")),
        RT::on_next(350, s("1 6")),
        RT::on_next(350, s("2 6"))
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 370));
}

#[test]
fn window_with_count_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = count_source(&scheduler, RT::on_error(600, ex.clone()));

    let source = xs.as_observable();
    let res = scheduler.start(move || labeled(source.window_with_skip(3, 2)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("0 4")),
        RT::on_next(280, s("1 4")),
        RT::on_next(320, s("1 5")),
        RT::on_next(350, s("1 6")),
        RT::on_next(350, s("2 6")),
        RT::on_next(380, s("2 7")),
        RT::on_next(420, s("2 8")),
        RT::on_next(420, s("3 8")),
        RT::on_next(470, s("3 9")),
        RT::on_error(600, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn window_time_basic() {
    let scheduler = TestScheduler::new();

    let xs = time_source(&scheduler, &TIME_VALUES, RT::on_completed(490));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || labeled_with_end(source.window_time_on(ticks_to_duration(100), sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(270, s("0 4")),
        RT::on_next(300, s("0 end")),
        RT::on_next(320, s("1 5")),
        RT::on_next(360, s("1 6")),
        RT::on_next(390, s("1 7")),
        RT::on_next(400, s("1 end")),
        RT::on_next(410, s("2 8")),
        RT::on_next(460, s("2 9")),
        RT::on_next(470, s("2 10")),
        RT::on_next(490, s("2 end")),
        RT::on_completed(490)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 490));
}

#[test]
fn window_time_basic_periodic() {
    let scheduler = TestScheduler::new();
    let periodic = PeriodicTestScheduler::new(&scheduler);

    let xs = time_source(&scheduler, &TIME_VALUES, RT::on_completed(490));

    let source = xs.as_observable();
    let sched: Rc<dyn IScheduler> = periodic.clone();
    let res = scheduler.start(move || labeled_with_end(source.window_time_on(ticks_to_duration(100), sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(270, s("0 4")),
        RT::on_next(300, s("0 end")),
        RT::on_next(320, s("1 5")),
        RT::on_next(360, s("1 6")),
        RT::on_next(390, s("1 7")),
        RT::on_next(400, s("1 end")),
        RT::on_next(410, s("2 8")),
        RT::on_next(460, s("2 9")),
        RT::on_next(470, s("2 10")),
        RT::on_next(490, s("2 end")),
        RT::on_completed(490)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 490));

    assert_eq!(vec![TimerRun::new(200, 490, vec![300, 400])], periodic.timers());
}

#[test]
fn window_time_basic_periodic_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();
    let periodic = PeriodicTestScheduler::new(&scheduler);

    let xs = time_source(&scheduler, &TIME_VALUES[..8], RT::on_error(460, ex.clone()));

    let source = xs.as_observable();
    let sched: Rc<dyn IScheduler> = periodic.clone();
    let res = scheduler.start(move || labeled_with_end(source.window_time_on(ticks_to_duration(100), sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(270, s("0 4")),
        RT::on_next(300, s("0 end")),
        RT::on_next(320, s("1 5")),
        RT::on_next(360, s("1 6")),
        RT::on_next(390, s("1 7")),
        RT::on_next(400, s("1 end")),
        RT::on_next(410, s("2 8")),
        RT::on_error(460, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 460));

    assert_eq!(vec![TimerRun::new(200, 460, vec![300, 400])], periodic.timers());
}

#[test]
fn window_time_basic_both() {
    let scheduler = TestScheduler::new();

    let xs = time_source(&scheduler, &TIME_VALUES, RT::on_completed(490));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || labeled_with_end(source.window_time_shift_on(ticks_to_duration(100), ticks_to_duration(50), sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(270, s("0 4")),
        RT::on_next(270, s("1 4")),
        RT::on_next(300, s("0 end")),
        RT::on_next(320, s("1 5")),
        RT::on_next(320, s("2 5")),
        RT::on_next(350, s("1 end")),
        RT::on_next(360, s("2 6")),
        RT::on_next(360, s("3 6")),
        RT::on_next(390, s("2 7")),
        RT::on_next(390, s("3 7")),
        RT::on_next(400, s("2 end")),
        RT::on_next(410, s("3 8")),
        RT::on_next(410, s("4 8")),
        RT::on_next(450, s("3 end")),
        RT::on_next(460, s("4 9")),
        RT::on_next(460, s("5 9")),
        RT::on_next(470, s("4 10")),
        RT::on_next(470, s("5 10")),
        RT::on_next(490, s("4 end")),
        RT::on_next(490, s("5 end")),
        RT::on_completed(490)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 490));
}

#[test]
fn window_with_time_basic1() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || labeled(source.window_time_shift_on(ticks_to_duration(100), ticks_to_duration(70), sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("0 4")),
        RT::on_next(280, s("1 4")),
        RT::on_next(320, s("1 5")),
        RT::on_next(350, s("1 6")),
        RT::on_next(350, s("2 6")),
        RT::on_next(380, s("2 7")),
        RT::on_next(420, s("2 8")),
        RT::on_next(420, s("3 8")),
        RT::on_next(470, s("3 9")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn window_with_time_basic2() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || labeled(source.window_time_shift_on(ticks_to_duration(70), ticks_to_duration(100), sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(320, s("1 5")),
        RT::on_next(350, s("1 6")),
        RT::on_next(420, s("2 8")),
        RT::on_next(470, s("2 9")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn window_with_time_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = count_source(&scheduler, RT::on_error(600, ex.clone()));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || labeled(source.window_time_shift_on(ticks_to_duration(100), ticks_to_duration(70), sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("0 4")),
        RT::on_next(280, s("1 4")),
        RT::on_next(320, s("1 5")),
        RT::on_next(350, s("1 6")),
        RT::on_next(350, s("2 6")),
        RT::on_next(380, s("2 7")),
        RT::on_next(420, s("2 8")),
        RT::on_next(420, s("3 8")),
        RT::on_next(470, s("3 9")),
        RT::on_error(600, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn window_with_time_disposed() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start_disposed(move || labeled(source.window_time_shift_on(ticks_to_duration(100), ticks_to_duration(70), sched)), 370);

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("0 4")),
        RT::on_next(280, s("1 4")),
        RT::on_next(320, s("1 5")),
        RT::on_next(350, s("1 6")),
        RT::on_next(350, s("2 6"))
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 370));
}

#[test]
fn window_with_time_basic_same() {
    let scheduler = TestScheduler::new();

    let xs = count_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || labeled(source.window_time_on(ticks_to_duration(100), sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("0 4")),
        RT::on_next(320, s("1 5")),
        RT::on_next(350, s("1 6")),
        RT::on_next(380, s("1 7")),
        RT::on_next(420, s("2 8")),
        RT::on_next(470, s("2 9")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn window_with_time_or_count_basic() {
    let scheduler = TestScheduler::new();

    let xs = time_or_count_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || labeled(source.window_time_count_on(ticks_to_duration(70), 3, sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(205, s("0 1")),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("1 4")),
        RT::on_next(320, s("2 5")),
        RT::on_next(350, s("2 6")),
        RT::on_next(370, s("2 7")),
        RT::on_next(420, s("3 8")),
        RT::on_next(470, s("4 9")),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn window_with_time_or_count_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = time_or_count_source(&scheduler, RT::on_error(600, ex.clone()));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || labeled(source.window_time_count_on(ticks_to_duration(70), 3, sched)));

    assert_messages!(
        res.messages(),
        RT::on_next(205, s("0 1")),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("1 4")),
        RT::on_next(320, s("2 5")),
        RT::on_next(350, s("2 6")),
        RT::on_next(370, s("2 7")),
        RT::on_next(420, s("3 8")),
        RT::on_next(470, s("4 9")),
        RT::on_error(600, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn window_with_time_or_count_disposed() {
    let scheduler = TestScheduler::new();

    let xs = time_or_count_source(&scheduler, RT::on_completed(600));

    let (source, sched) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start_disposed(move || labeled(source.window_time_count_on(ticks_to_duration(70), 3, sched)), 370);

    assert_messages!(
        res.messages(),
        RT::on_next(205, s("0 1")),
        RT::on_next(210, s("0 2")),
        RT::on_next(240, s("0 3")),
        RT::on_next(280, s("1 4")),
        RT::on_next(320, s("2 5")),
        RT::on_next(350, s("2 6")),
        RT::on_next(370, s("2 7"))
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 370));
}

// Not ported: Window_ArgumentChecking - argument validation
// Not ported: Window_Closings_Basic - needs operator Timer (phase 3, later)
// Not ported: Window_Closings_Empty - needs operator Delay (phase 3, later)
// Not ported: Window_Closings_Dispose - needs operator Timer (phase 3, later)
// Not ported: Window_Closings_Error - needs operator Timer (phase 3, later)
// Not ported: Window_Closings_Default - needs operator Timer (phase 3, later)
// Not ported: Window_OpeningClosings_Basic - needs operators GroupJoin (Window by openings), Timer (phase 3, later)
// Not ported: Window_OpeningClosings_Throw - needs operator GroupJoin (Window by openings) (phase 3, later)
// Not ported: Window_OpeningClosings_Dispose - needs operators GroupJoin (Window by openings), Timer (phase 3, later)
// Not ported: Window_OpeningClosings_Data_Error - needs operators GroupJoin (Window by openings), Timer (phase 3, later)
// Not ported: Window_OpeningClosings_Window_Error - needs operators GroupJoin (Window by openings), Timer (phase 3, later)
// Not ported: WindowWithCount_ArgumentChecking - argument validation
// Not ported: WindowWithCount_Default - needs operators Range, Skip, First, SequenceEqual (phase 3, later)
// Not ported: WindowWithTime_ArgumentChecking - argument validation
// Not ported: WindowWithTime_Default - needs operators Range, SelectMany, ToList, First (phase 3, later)
// Not ported: WindowWithTimeOrCount_ArgumentChecking - argument validation
// Not ported: WindowWithTimeOrCount_Default - needs operators Range, Skip, First, SequenceEqual (phase 3, later)
