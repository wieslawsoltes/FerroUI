//! Ported from `Tests.System.Reactive/Tests/Linq/Subjects/ReplaySubjectTest.cs` of Rx.NET.
//! Adapted: the variables upstream's scheduled lambdas assign (`subject`, `subscription`,
//! `subscription1`, ...) are shared slots of a small harness (`Harness`), one call per upstream
//! `ScheduleAbsolute`; `TimeSpan.FromTicks(n)` is `ticks_to_duration(n)`, `ObjectDisposedException`
//! is a panic checked with `assert_throws`.

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction};
use ferroui_reactive::subjects::{ReplaySubject, SubjectBase};
use ferroui_reactive::{IDisposable, IObservable, IObserver, NotificationKind, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_throws, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// A variable that a scheduled action assigns and a later one reads.
type Slot<T> = Rc<RefCell<Option<T>>>;

fn slot<T>() -> Slot<T> {
    Rc::new(RefCell::new(None))
}

fn get<T: Clone>(slot: &Slot<T>) -> T {
    slot.borrow().clone().expect("the variable is assigned")
}

/// The `subject` variable of upstream's scheduled tests and the actions scheduled on it.
struct Harness {
    scheduler: TestScheduler,
    subject: Slot<Rc<ReplaySubject<i32>>>,
}

impl Harness {
    fn new(scheduler: &TestScheduler) -> Self {
        Self { scheduler: scheduler.clone(), subject: slot() }
    }

    /// `scheduler.ScheduleAbsolute(time, () => subject = create())`.
    fn create_at(&self, time: i64, create: impl FnOnce() -> Rc<ReplaySubject<i32>> + 'static) {
        let subject = self.subject.clone();
        self.scheduler.schedule_absolute_action(time, move || *subject.borrow_mut() = Some(create()));
    }

    /// `scheduler.ScheduleAbsolute(time, () => subscription = xs.Subscribe(subject))`.
    fn connect_at(&self, time: i64, xs: Rc<dyn IObservable<i32>>) -> Slot<Rc<dyn IDisposable>> {
        let subject = self.subject.clone();
        let subscription = slot();
        let assigned = subscription.clone();
        self.scheduler.schedule_absolute_action(time, move || {
            let subject = get(&subject);
            let d = xs.subscribe(subject.as_observer());
            *assigned.borrow_mut() = Some(d);
        });
        subscription
    }

    /// `scheduler.ScheduleAbsolute(time, () => subscriptionN = subject.Subscribe(resultsN))`.
    fn subscribe_at(&self, time: i64, observer: Rc<dyn IObserver<i32>>) -> Slot<Rc<dyn IDisposable>> {
        let subject = self.subject.clone();
        let subscription = slot();
        let assigned = subscription.clone();
        self.scheduler.schedule_absolute_action(time, move || {
            let subject = get(&subject);
            let d = subject.subscribe(observer);
            *assigned.borrow_mut() = Some(d);
        });
        subscription
    }

    /// `scheduler.ScheduleAbsolute(time, () => subscriptionN.Dispose())`.
    fn dispose_at(&self, time: i64, subscription: &Slot<Rc<dyn IDisposable>>) {
        let subscription = subscription.clone();
        self.scheduler.schedule_absolute_action(time, move || get(&subscription).dispose());
    }

    /// `scheduler.ScheduleAbsolute(time, () => action(subject))`.
    fn act_at(&self, time: i64, action: impl FnOnce(Rc<ReplaySubject<i32>>) + 'static) {
        let subject = self.subject.clone();
        self.scheduler.schedule_absolute_action(time, move || action(get(&subject)));
    }
}

/// A scheduler that schedules nothing (`DummyScheduler` of upstream's test dummies).
struct DummyScheduler;

impl IScheduler for DummyScheduler {
    fn now(&self) -> SystemTime {
        // `DateTimeOffset.MinValue` upstream.
        SystemTime::UNIX_EPOCH
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
fn constructor_argument_checking() {
    // The negative sizes and windows, and the null schedulers, of upstream cannot be expressed
    // (`usize`, `Duration`, `Rc`); what remains is that zero is allowed.
    let dummy: Rc<dyn IScheduler> = Rc::new(DummyScheduler);

    let _ = ReplaySubject::<i32>::with_buffer_size(0);
    let _ = ReplaySubject::<i32>::with_window(Duration::ZERO);
    let _ = ReplaySubject::<i32>::with_buffer_size_and_window(0, Duration::ZERO);
    let _ = ReplaySubject::<i32>::with_buffer_size_and_scheduler(0, dummy.clone());
    let _ = ReplaySubject::<i32>::with_window_and_scheduler(Duration::ZERO, dummy.clone());
    let _ = ReplaySubject::<i32>::with_buffer_size_window_and_scheduler(0, Duration::ZERO, dummy);
}

#[test]
fn infinite_replay_by_time() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_next(630, 8),
        RT::on_next(710, 9),
        RT::on_next(870, 10),
        RT::on_next(940, 11),
        RT::on_next(1020, 12),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    let s = scheduler.scheduler();
    h.create_at(100, move || ReplaySubject::with_buffer_size_window_and_scheduler(3, ticks_to_duration(100), s));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(301, 3),
        RT::on_next(302, 4),
        RT::on_next(341, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(401, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
        RT::on_next(631, 8),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(901, 10),
        RT::on_next(941, 11),
    );
}

#[test]
fn infinite_replay_one() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_next(630, 8),
        RT::on_next(710, 9),
        RT::on_next(870, 10),
        RT::on_next(940, 11),
        RT::on_next(1020, 12),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();
    let results4 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(1));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1200, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());
    let _subscription4 = h.subscribe_at(1100, results4.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(300, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_next(630, 8),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(900, 10),
        RT::on_next(940, 11),
    );

    assert_messages!(
        results4.messages(),
        RT::on_next(1100, 12),
    );
}

#[test]
fn infinite_replay_many() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_next(630, 8),
        RT::on_next(710, 9),
        RT::on_next(870, 10),
        RT::on_next(940, 11),
        RT::on_next(1020, 12),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(3));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(300, 3),
        RT::on_next(300, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(400, 3),
        RT::on_next(400, 4),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_next(630, 8),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(900, 8),
        RT::on_next(900, 9),
        RT::on_next(900, 10),
        RT::on_next(940, 11),
    );
}

#[test]
fn infinite_replay_all() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_next(630, 8),
        RT::on_next(710, 9),
        RT::on_next(870, 10),
        RT::on_next(940, 11),
        RT::on_next(1020, 12),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::new());
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(300, 3),
        RT::on_next(300, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(400, 3),
        RT::on_next(400, 4),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_next(630, 8),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(900, 3),
        RT::on_next(900, 4),
        RT::on_next(900, 5),
        RT::on_next(900, 6),
        RT::on_next(900, 7),
        RT::on_next(900, 8),
        RT::on_next(900, 9),
        RT::on_next(900, 10),
        RT::on_next(940, 11),
    );
}

#[test]
fn infinite2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(280, -1),
        RT::on_next(290, -2),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_next(630, 8),
        RT::on_next(710, 9),
        RT::on_next(870, 10),
        RT::on_next(940, 11),
        RT::on_next(1020, 12),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    let s = scheduler.scheduler();
    h.create_at(100, move || ReplaySubject::with_buffer_size_window_and_scheduler(3, ticks_to_duration(100), s));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(301, 4),
        RT::on_next(302, -1),
        RT::on_next(303, -2),
        RT::on_next(341, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(401, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
        RT::on_next(631, 8),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(901, 10),
        RT::on_next(941, 11),
    );
}

#[test]
fn finite_replay_by_time() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_completed(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    let s = scheduler.scheduler();
    h.create_at(100, move || ReplaySubject::with_buffer_size_window_and_scheduler(3, ticks_to_duration(100), s));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(301, 3),
        RT::on_next(302, 4),
        RT::on_next(341, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(401, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
        RT::on_completed::<i32>(631),
    );

    assert_messages!(
        results3.messages(),
        RT::on_completed::<i32>(901),
    );
}

#[test]
fn finite_replay_one() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_completed(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(1));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(300, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_completed::<i32>(630),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(900, 7),
        RT::on_completed::<i32>(900),
    );
}

#[test]
fn finite_replay_many() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_completed(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(3));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(300, 3),
        RT::on_next(300, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(400, 3),
        RT::on_next(400, 4),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_completed::<i32>(630),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(900, 5),
        RT::on_next(900, 6),
        RT::on_next(900, 7),
        RT::on_completed::<i32>(900),
    );
}

#[test]
fn finite_replay_all() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_completed(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::new());
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(300, 3),
        RT::on_next(300, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(400, 3),
        RT::on_next(400, 4),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_completed::<i32>(630),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(900, 3),
        RT::on_next(900, 4),
        RT::on_next(900, 5),
        RT::on_next(900, 6),
        RT::on_next(900, 7),
        RT::on_completed::<i32>(900),
    );
}

#[test]
fn error_replay_by_time() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_error(630, ex.clone()),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    let s = scheduler.scheduler();
    h.create_at(100, move || ReplaySubject::with_buffer_size_window_and_scheduler(3, ticks_to_duration(100), s));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(301, 3),
        RT::on_next(302, 4),
        RT::on_next(341, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(401, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
        RT::on_error::<i32>(631, ex.clone()),
    );

    assert_messages!(
        results3.messages(),
        RT::on_error::<i32>(901, ex.clone()),
    );
}

#[test]
fn error_replay_one() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_error(630, ex.clone()),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(1));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(300, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_error::<i32>(630, ex.clone()),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(900, 7),
        RT::on_error::<i32>(900, ex.clone()),
    );
}

#[test]
fn error_replay_many() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_error(630, ex.clone()),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(3));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(300, 3),
        RT::on_next(300, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(400, 3),
        RT::on_next(400, 4),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_error::<i32>(630, ex.clone()),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(900, 5),
        RT::on_next(900, 6),
        RT::on_next(900, 7),
        RT::on_error::<i32>(900, ex.clone()),
    );
}

#[test]
fn error_replay_all() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_error(630, ex.clone()),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::new());
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(300, 3),
        RT::on_next(300, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(400, 3),
        RT::on_next(400, 4),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_error::<i32>(630, ex.clone()),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(900, 3),
        RT::on_next(900, 4),
        RT::on_next(900, 5),
        RT::on_next(900, 6),
        RT::on_next(900, 7),
        RT::on_error::<i32>(900, ex.clone()),
    );
}

#[test]
fn canceled_replay_by_time() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_completed::<i32>(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    let s = scheduler.scheduler();
    h.create_at(100, move || ReplaySubject::with_buffer_size_window_and_scheduler(3, ticks_to_duration(100), s));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(results1.messages());

    assert_messages!(
        results2.messages(),
        RT::on_completed::<i32>(631),
    );

    assert_messages!(
        results3.messages(),
        RT::on_completed::<i32>(901),
    );
}

#[test]
fn canceled_replay_one() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_completed::<i32>(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(1));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(results1.messages());

    assert_messages!(
        results2.messages(),
        RT::on_completed::<i32>(630),
    );

    assert_messages!(
        results3.messages(),
        RT::on_completed::<i32>(900),
    );
}

#[test]
fn canceled_replay_many() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_completed::<i32>(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(3));
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(results1.messages());

    assert_messages!(
        results2.messages(),
        RT::on_completed::<i32>(630),
    );

    assert_messages!(
        results3.messages(),
        RT::on_completed::<i32>(900),
    );
}

#[test]
fn canceled_replay_all() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_completed::<i32>(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::new());
    let subscription = h.connect_at(200, xs.as_observable());
    h.dispose_at(1000, &subscription);

    let subscription1 = h.subscribe_at(300, results1.clone());
    let subscription2 = h.subscribe_at(400, results2.clone());
    let subscription3 = h.subscribe_at(900, results3.clone());

    h.dispose_at(600, &subscription1);
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription1);
    h.dispose_at(950, &subscription3);

    scheduler.run();

    assert_messages!(results1.messages());

    assert_messages!(
        results2.messages(),
        RT::on_completed::<i32>(630),
    );

    assert_messages!(
        results3.messages(),
        RT::on_completed::<i32>(900),
    );
}

#[test]
fn subject_disposed() {
    let scheduler = TestScheduler::new();

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    let s = scheduler.scheduler();
    h.create_at(100, move || ReplaySubject::with_scheduler(s));
    let subscription1 = h.subscribe_at(200, results1.clone());
    let subscription2 = h.subscribe_at(300, results2.clone());
    let subscription3 = h.subscribe_at(400, results3.clone());
    h.dispose_at(500, &subscription1);
    h.act_at(600, |s| s.dispose());
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription3);

    h.act_at(150, |s| s.on_next(1));
    h.act_at(250, |s| s.on_next(2));
    h.act_at(350, |s| s.on_next(3));
    h.act_at(450, |s| s.on_next(4));
    h.act_at(550, |s| s.on_next(5));
    h.act_at(650, |s| assert_throws(|| s.on_next(6)));
    h.act_at(750, |s| assert_throws(|| s.on_completed()));
    h.act_at(850, |s| assert_throws(|| s.on_error(new_error("ex"))));
    h.act_at(950, |s| {
        assert_throws(|| {
            s.as_observable().subscribe_ignore();
        })
    });

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(201, 1),
        RT::on_next(251, 2),
        RT::on_next(351, 3),
        RT::on_next(451, 4),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(301, 1),
        RT::on_next(302, 2),
        RT::on_next(351, 3),
        RT::on_next(451, 4),
        RT::on_next(551, 5),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(401, 1),
        RT::on_next(402, 2),
        RT::on_next(403, 3),
        RT::on_next(451, 4),
        RT::on_next(551, 5),
    );
}

#[test]
fn subject_disposed_replay_one() {
    let scheduler = TestScheduler::new();

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(1));
    let subscription1 = h.subscribe_at(200, results1.clone());
    let subscription2 = h.subscribe_at(300, results2.clone());
    let subscription3 = h.subscribe_at(400, results3.clone());
    h.dispose_at(500, &subscription1);
    h.act_at(600, |s| s.dispose());
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription3);

    h.act_at(150, |s| s.on_next(1));
    h.act_at(250, |s| s.on_next(2));
    h.act_at(350, |s| s.on_next(3));
    h.act_at(450, |s| s.on_next(4));
    h.act_at(550, |s| s.on_next(5));
    h.act_at(650, |s| assert_throws(|| s.on_next(6)));
    h.act_at(750, |s| assert_throws(|| s.on_completed()));
    h.act_at(850, |s| assert_throws(|| s.on_error(new_error("ex"))));
    h.act_at(950, |s| {
        assert_throws(|| {
            s.as_observable().subscribe_ignore();
        })
    });

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(200, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(300, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_next(550, 5),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(400, 3),
        RT::on_next(450, 4),
        RT::on_next(550, 5),
    );
}

#[test]
fn subject_disposed_replay_many() {
    let scheduler = TestScheduler::new();

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::with_buffer_size(3));
    let subscription1 = h.subscribe_at(200, results1.clone());
    let subscription2 = h.subscribe_at(300, results2.clone());
    let subscription3 = h.subscribe_at(400, results3.clone());
    h.dispose_at(500, &subscription1);
    h.act_at(600, |s| s.dispose());
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription3);

    h.act_at(150, |s| s.on_next(1));
    h.act_at(250, |s| s.on_next(2));
    h.act_at(350, |s| s.on_next(3));
    h.act_at(450, |s| s.on_next(4));
    h.act_at(550, |s| s.on_next(5));
    h.act_at(650, |s| assert_throws(|| s.on_next(6)));
    h.act_at(750, |s| assert_throws(|| s.on_completed()));
    h.act_at(850, |s| assert_throws(|| s.on_error(new_error("ex"))));
    h.act_at(950, |s| {
        assert_throws(|| {
            s.as_observable().subscribe_ignore();
        })
    });

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(200, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(300, 1),
        RT::on_next(300, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_next(550, 5),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(400, 1),
        RT::on_next(400, 2),
        RT::on_next(400, 3),
        RT::on_next(450, 4),
        RT::on_next(550, 5),
    );
}

#[test]
fn subject_disposed_replay_all() {
    let scheduler = TestScheduler::new();

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();

    h.create_at(100, || ReplaySubject::new());
    let subscription1 = h.subscribe_at(200, results1.clone());
    let subscription2 = h.subscribe_at(300, results2.clone());
    let subscription3 = h.subscribe_at(400, results3.clone());
    h.dispose_at(500, &subscription1);
    h.act_at(600, |s| s.dispose());
    h.dispose_at(700, &subscription2);
    h.dispose_at(800, &subscription3);

    h.act_at(150, |s| s.on_next(1));
    h.act_at(250, |s| s.on_next(2));
    h.act_at(350, |s| s.on_next(3));
    h.act_at(450, |s| s.on_next(4));
    h.act_at(550, |s| s.on_next(5));
    h.act_at(650, |s| assert_throws(|| s.on_next(6)));
    h.act_at(750, |s| assert_throws(|| s.on_completed()));
    h.act_at(850, |s| assert_throws(|| s.on_error(new_error("ex"))));
    h.act_at(950, |s| {
        assert_throws(|| {
            s.as_observable().subscribe_ignore();
        })
    });

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(200, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(300, 1),
        RT::on_next(300, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_next(550, 5),
    );

    assert_messages!(
        results3.messages(),
        RT::on_next(400, 1),
        RT::on_next(400, 2),
        RT::on_next(400, 3),
        RT::on_next(450, 4),
        RT::on_next(550, 5),
    );
}

#[test]
fn replay_subject_dies_out() {
    //
    // Tests v1.x behavior as documented in ReplaySubject.cs (Subscribe method).
    //

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_completed(580),
    ]);

    let h = Harness::new(&scheduler);

    let results1 = scheduler.create_observer::<i32>();
    let results2 = scheduler.create_observer::<i32>();
    let results3 = scheduler.create_observer::<i32>();
    let results4 = scheduler.create_observer::<i32>();

    let s = scheduler.scheduler();
    h.create_at(100, move || ReplaySubject::with_buffer_size_window_and_scheduler(i32::MAX as usize, ticks_to_duration(100), s));
    h.connect_at(200, xs.as_observable());

    h.subscribe_at(300, results1.clone());
    h.subscribe_at(400, results2.clone());
    h.subscribe_at(600, results3.clone());
    h.subscribe_at(900, results4.clone());

    scheduler.run();

    assert_messages!(
        results1.messages(),
        RT::on_next(301, 3),
        RT::on_next(302, 4),
        RT::on_next(341, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
        RT::on_completed::<i32>(581),
    );

    assert_messages!(
        results2.messages(),
        RT::on_next(401, 5),
        RT::on_next(411, 6),
        RT::on_next(521, 7),
        RT::on_completed::<i32>(581),
    );

    assert_messages!(results3.messages(), RT::on_next(601, 7), RT::on_completed::<i32>(602));

    assert_messages!(results4.messages(), RT::on_completed::<i32>(901));
}

#[test]
fn has_observers() {
    has_observers_impl(ReplaySubject::new());
    has_observers_impl(ReplaySubject::with_buffer_size(1));
    has_observers_impl(ReplaySubject::with_buffer_size(3));
    has_observers_impl(ReplaySubject::with_window(Duration::from_secs(1)));
}

fn has_observers_impl(s: Rc<ReplaySubject<i32>>) {
    assert!(!s.has_observers());

    let d1 = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());

    d1.dispose();
    assert!(!s.has_observers());

    let d2 = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());

    let d3 = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());

    d2.dispose();
    assert!(s.has_observers());

    d3.dispose();
    assert!(!s.has_observers());
}

#[test]
fn has_observers_dispose1() {
    has_observers_dispose1_impl(ReplaySubject::new());
    has_observers_dispose1_impl(ReplaySubject::with_buffer_size(1));
    has_observers_dispose1_impl(ReplaySubject::with_buffer_size(3));
    has_observers_dispose1_impl(ReplaySubject::with_window(Duration::from_secs(1)));
}

fn has_observers_dispose1_impl(s: Rc<ReplaySubject<i32>>) {
    assert!(!s.has_observers());
    assert!(!s.is_disposed());

    let d = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());
    assert!(!s.is_disposed());

    s.dispose();
    assert!(!s.has_observers());
    assert!(s.is_disposed());

    d.dispose();
    assert!(!s.has_observers());
    assert!(s.is_disposed());
}

#[test]
fn has_observers_dispose2() {
    has_observers_dispose2_impl(ReplaySubject::new());
    has_observers_dispose2_impl(ReplaySubject::with_buffer_size(1));
    has_observers_dispose2_impl(ReplaySubject::with_buffer_size(3));
    has_observers_dispose2_impl(ReplaySubject::with_window(Duration::from_secs(1)));
}

fn has_observers_dispose2_impl(s: Rc<ReplaySubject<i32>>) {
    assert!(!s.has_observers());
    assert!(!s.is_disposed());

    let d = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());
    assert!(!s.is_disposed());

    d.dispose();
    assert!(!s.has_observers());
    assert!(!s.is_disposed());

    s.dispose();
    assert!(!s.has_observers());
    assert!(s.is_disposed());
}

#[test]
fn has_observers_dispose3() {
    has_observers_dispose3_impl(ReplaySubject::new());
    has_observers_dispose3_impl(ReplaySubject::with_buffer_size(1));
    has_observers_dispose3_impl(ReplaySubject::with_buffer_size(3));
    has_observers_dispose3_impl(ReplaySubject::with_window(Duration::from_secs(1)));
}

fn has_observers_dispose3_impl(s: Rc<ReplaySubject<i32>>) {
    assert!(!s.has_observers());
    assert!(!s.is_disposed());

    s.dispose();
    assert!(!s.has_observers());
    assert!(s.is_disposed());
}

#[test]
fn has_observers_on_completed() {
    has_observers_on_completed_impl(ReplaySubject::new());
    has_observers_on_completed_impl(ReplaySubject::with_buffer_size(1));
    has_observers_on_completed_impl(ReplaySubject::with_buffer_size(3));
    has_observers_on_completed_impl(ReplaySubject::with_window(Duration::from_secs(1)));
}

fn has_observers_on_completed_impl(s: Rc<ReplaySubject<i32>>) {
    assert!(!s.has_observers());

    let _d = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());

    s.on_next(42);
    assert!(s.has_observers());

    s.on_completed();
    assert!(!s.has_observers());
}

#[test]
fn has_observers_on_error() {
    has_observers_on_error_impl(ReplaySubject::new());
    has_observers_on_error_impl(ReplaySubject::with_buffer_size(1));
    has_observers_on_error_impl(ReplaySubject::with_buffer_size(3));
    has_observers_on_error_impl(ReplaySubject::with_window(Duration::from_secs(1)));
}

fn has_observers_on_error_impl(s: Rc<ReplaySubject<i32>>) {
    assert!(!s.has_observers());

    let _d = s.as_observable().subscribe_next_error(|_| {}, |_| {});
    assert!(s.has_observers());

    s.on_next(42);
    assert!(s.has_observers());

    s.on_error(new_error("ex"));
    assert!(!s.has_observers());
}

#[test]
fn completed_to_late_subscriber_replay_all() {
    let s = ReplaySubject::<i32>::new();
    s.on_next(1);
    s.on_next(2);
    s.on_completed();

    let scheduler = TestScheduler::new();
    let observer = scheduler.create_observer::<i32>();
    s.subscribe(observer.clone());

    let messages = observer.messages();
    assert_eq!(3, messages.len());

    assert_eq!(Some(&1), messages[0].value.value());
    assert_eq!(Some(&2), messages[1].value.value());
    assert_eq!(NotificationKind::OnCompleted, messages[2].value.kind());
}

#[test]
fn completed_to_late_subscriber_replay_one() {
    let s = ReplaySubject::<i32>::with_buffer_size(1);
    s.on_next(1);
    s.on_next(2);
    s.on_completed();

    let scheduler = TestScheduler::new();
    let observer = scheduler.create_observer::<i32>();
    s.subscribe(observer.clone());

    let messages = observer.messages();
    assert_eq!(2, messages.len());

    assert_eq!(Some(&2), messages[0].value.value());
    assert_eq!(NotificationKind::OnCompleted, messages[1].value.kind());
}

#[test]
fn completed_to_late_subscriber_replay_many() {
    let s = ReplaySubject::<i32>::with_buffer_size(2);
    s.on_next(1);
    s.on_next(2);
    s.on_next(3);
    s.on_completed();

    let scheduler = TestScheduler::new();
    let observer = scheduler.create_observer::<i32>();
    s.subscribe(observer.clone());

    let messages = observer.messages();
    assert_eq!(3, messages.len());

    assert_eq!(Some(&2), messages[0].value.value());
    assert_eq!(Some(&3), messages[1].value.value());
    assert_eq!(NotificationKind::OnCompleted, messages[2].value.kind());
}

#[test]
fn completed_to_late_subscriber_replay_by_time() {
    let s = ReplaySubject::<i32>::with_window(Duration::from_secs(60));
    s.on_next(1);
    s.on_next(2);
    s.on_next(3);
    s.on_completed();

    let scheduler = TestScheduler::new();
    let observer = scheduler.create_observer::<i32>();
    s.subscribe(observer.clone());

    let messages = observer.messages();
    assert_eq!(4, messages.len());

    assert_eq!(Some(&1), messages[0].value.value());
    assert_eq!(Some(&2), messages[1].value.value());
    assert_eq!(Some(&3), messages[2].value.value());
    assert_eq!(NotificationKind::OnCompleted, messages[3].value.kind());
}

fn same_error(actual: Option<&ferroui_reactive::ObservableError>, expected: &ferroui_reactive::ObservableError) -> bool {
    actual.is_some_and(|actual| std::ptr::addr_eq(Rc::as_ptr(actual), Rc::as_ptr(expected)))
}

#[test]
fn errored_to_late_subscriber_replay_all() {
    let expected_exception = new_error("Test");
    let s = ReplaySubject::<i32>::new();
    s.on_next(1);
    s.on_next(2);
    s.on_error(expected_exception.clone());

    let scheduler = TestScheduler::new();
    let observer = scheduler.create_observer::<i32>();
    s.subscribe(observer.clone());

    let messages = observer.messages();
    assert_eq!(3, messages.len());

    assert_eq!(Some(&1), messages[0].value.value());
    assert_eq!(Some(&2), messages[1].value.value());
    assert_eq!(NotificationKind::OnError, messages[2].value.kind());
    assert!(same_error(messages[2].value.exception(), &expected_exception));
}

#[test]
fn errored_to_late_subscriber_replay_one() {
    let expected_exception = new_error("Test");
    let s = ReplaySubject::<i32>::with_buffer_size(1);
    s.on_next(1);
    s.on_next(2);
    s.on_error(expected_exception.clone());

    let scheduler = TestScheduler::new();
    let observer = scheduler.create_observer::<i32>();
    s.subscribe(observer.clone());

    let messages = observer.messages();
    assert_eq!(2, messages.len());

    assert_eq!(Some(&2), messages[0].value.value());
    assert_eq!(NotificationKind::OnError, messages[1].value.kind());
    assert!(same_error(messages[1].value.exception(), &expected_exception));
}

#[test]
fn errored_to_late_subscriber_replay_many() {
    let expected_exception = new_error("Test");
    let s = ReplaySubject::<i32>::with_buffer_size(2);
    s.on_next(1);
    s.on_next(2);
    s.on_next(3);
    s.on_error(expected_exception.clone());

    let scheduler = TestScheduler::new();
    let observer = scheduler.create_observer::<i32>();
    s.subscribe(observer.clone());

    let messages = observer.messages();
    assert_eq!(3, messages.len());

    assert_eq!(Some(&2), messages[0].value.value());
    assert_eq!(Some(&3), messages[1].value.value());
    assert_eq!(NotificationKind::OnError, messages[2].value.kind());
    assert!(same_error(messages[2].value.exception(), &expected_exception));
}

#[test]
fn errored_to_late_subscriber_replay_by_time() {
    let expected_exception = new_error("Test");
    let s = ReplaySubject::<i32>::with_window(Duration::from_secs(60));
    s.on_next(1);
    s.on_next(2);
    s.on_next(3);
    s.on_error(expected_exception.clone());

    let scheduler = TestScheduler::new();
    let observer = scheduler.create_observer::<i32>();
    s.subscribe(observer.clone());

    let messages = observer.messages();
    assert_eq!(4, messages.len());

    assert_eq!(Some(&1), messages[0].value.value());
    assert_eq!(Some(&2), messages[1].value.value());
    assert_eq!(Some(&3), messages[2].value.value());
    assert_eq!(NotificationKind::OnError, messages[3].value.kind());
    assert!(same_error(messages[3].value.exception(), &expected_exception));
}

#[test]
fn replay_subject_reentrant() {
    let r = ReplaySubject::<i32>::with_buffer_size(4);

    r.on_next(0);
    r.on_next(1);
    r.on_next(2);
    r.on_next(3);
    r.on_next(4);

    let xs = Rc::new(RefCell::new(Vec::new()));

    let i = Rc::new(Cell::new(0));
    {
        let (xs, i, r2) = (xs.clone(), i.clone(), r.clone());
        r.as_observable().subscribe_next(move |x| {
            xs.borrow_mut().push(x);

            i.set(i.get() + 1);
            if i.get() <= 10 {
                r2.on_next(x);
            }
        });
    }

    r.on_next(5);

    assert_eq!(
        vec![
            1, 2, 3, 4, // original
            1, 2, 3, 4, // reentrant (+ fed back)
            1, 2, 3, 4, // reentrant (+ first two fed back)
            1, 2, // reentrant
            5, // tune in
        ],
        *xs.borrow()
    );
}

// Not ported: Subscribe_ArgumentChecking - argument validation
// Not ported: OnError_ArgumentChecking - argument validation
// Not ported: FastImmediateObserver_Simple1 - FastImmediateObserver is internal to the crate (pub(crate) module internal); belongs in the crate's own tests
// Not ported: FastImmediateObserver_Simple2 - FastImmediateObserver is internal to the crate (pub(crate) module internal); belongs in the crate's own tests
// Not ported: FastImmediateObserver_Simple3 - FastImmediateObserver is internal to the crate (pub(crate) module internal); belongs in the crate's own tests
// Not ported: FastImmediateObserver_Fault - FastImmediateObserver is internal to the crate (pub(crate) module internal); belongs in the crate's own tests
// Not ported: FastImmediateObserver_Ownership1 - real threads
// Not ported: FastImmediateObserver_Ownership2 - real threads
