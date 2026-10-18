//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ReplayTest.cs` of Rx.NET.
//! Adapted: the variables the scheduled actions assign are shared slots, set up by
//! `run_connectable` in the order of the upstream tests; the overloads are `replay_buffer` (count),
//! `replay_window` (time) and `replay_with*` (selector), with `_on` for the scheduler.
//! `DefaultScheduler.Instance` is the default scheduler of the thread
//! (`SchedulerDefaults::default_scheduler`, the current-thread scheduler), and `AssertEqual` of two
//! sequences compares their notifications, collected during the subscription.

use ferroui_reactive::concurrency::SchedulerDefaults;
use ferroui_reactive::subjects::{IConnectableObservable, Subject};
use ferroui_reactive::{IDisposable, IObservable, IObserver, Notification, Observable, ObservableExt};
use ferroui_reactive_testing::{
    assert_messages, assert_subscriptions, new_error, ticks_to_duration, MockObserver, ReactiveTest as RT, Recorded, TestScheduler,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

type Slot<T> = Rc<RefCell<Option<T>>>;

fn slot<T>() -> Slot<T> {
    Rc::new(RefCell::new(None))
}

fn get<T: Clone>(slot: &Slot<T>) -> T {
    slot.borrow().clone().expect("the slot is assigned")
}

/// The messages of the hot source of the tests, ending with `last`.
fn source_messages(last: Recorded<Notification<i32>>) -> Vec<Recorded<Notification<i32>>> {
    vec![
        RT::on_next(110, 7),
        RT::on_next(220, 3),
        RT::on_next(280, 4),
        RT::on_next(290, 1),
        RT::on_next(340, 8),
        RT::on_next(360, 5),
        RT::on_next(370, 6),
        RT::on_next(390, 7),
        RT::on_next(410, 13),
        RT::on_next(430, 2),
        RT::on_next(450, 9),
        RT::on_next(520, 11),
        RT::on_next(560, 20),
        last,
    ]
}

/// Creates the connectable observable at `Created`, subscribes an observer at `subscribed` and
/// disposes it at `disposed`, connects and disconnects at each pair of `connections`, and runs the
/// scheduler (the `ScheduleAbsolute` calls of the upstream tests).
fn run_connectable(
    scheduler: &TestScheduler,
    create: impl FnOnce() -> Rc<dyn IConnectableObservable<i32>> + 'static,
    subscribed: i64,
    disposed: i64,
    connections: &[(i64, i64)],
) -> Rc<MockObserver<i32>> {
    let ys: Slot<Rc<dyn IConnectableObservable<i32>>> = slot();
    let subscription: Slot<Rc<dyn IDisposable>> = slot();
    let connection: Slot<Rc<dyn IDisposable>> = slot();
    let res = scheduler.create_observer::<i32>();

    {
        let ys = ys.clone();
        scheduler.schedule_absolute_action(RT::CREATED, move || *ys.borrow_mut() = Some(create()));
    }
    {
        let (ys, subscription, res) = (ys.clone(), subscription.clone(), res.clone());
        scheduler.schedule_absolute_action(subscribed, move || {
            let connectable = get(&ys);
            let source: Rc<dyn IObservable<i32>> = connectable;
            *subscription.borrow_mut() = Some(source.subscribe(res));
        });
    }
    {
        let subscription = subscription.clone();
        scheduler.schedule_absolute_action(disposed, move || get(&subscription).dispose());
    }

    for &(connect, disconnect) in connections {
        {
            let (ys, connection) = (ys.clone(), connection.clone());
            scheduler.schedule_absolute_action(connect, move || *connection.borrow_mut() = Some(get(&ys).connect()));
        }
        {
            let connection = connection.clone();
            scheduler.schedule_absolute_action(disconnect, move || get(&connection).dispose());
        }
    }

    scheduler.run();

    res
}

/// The notifications of `source`, which ends during the subscription.
fn notifications(source: &impl ObservableExt<i32>) -> Vec<Notification<i32>> {
    let list = Rc::new(RefCell::new(Vec::new()));
    {
        let list = list.clone();
        source.materialize().subscribe_next(move |n| list.borrow_mut().push(n));
    }
    let result = list.borrow().clone();
    result
}

#[test]
fn replay_count_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = run_connectable(
        &scheduler,
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_buffer_on(3, scheduler)
        },
        450,
        RT::DISPOSED,
        &[(300, 400), (500, 550), (650, 800)],
    );

    assert_messages!(
        res.messages(),
        RT::on_next(451, 5),
        RT::on_next(452, 6),
        RT::on_next(453, 7),
        RT::on_next(521, 11),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 550), RT::subscribe(650, 800));
}

#[test]
fn replay_count_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(source_messages(RT::on_error(600, ex.clone())));

    let res = run_connectable(
        &scheduler,
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_buffer_on(3, scheduler)
        },
        450,
        RT::DISPOSED,
        &[(300, 400), (500, 800)],
    );

    assert_messages!(
        res.messages(),
        RT::on_next(451, 5),
        RT::on_next(452, 6),
        RT::on_next(453, 7),
        RT::on_next(521, 11),
        RT::on_next(561, 20),
        RT::on_error::<i32>(601, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 600));
}

#[test]
fn replay_count_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = run_connectable(
        &scheduler,
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_buffer_on(3, scheduler)
        },
        450,
        RT::DISPOSED,
        &[(300, 400), (500, 800)],
    );

    assert_messages!(
        res.messages(),
        RT::on_next(451, 5),
        RT::on_next(452, 6),
        RT::on_next(453, 7),
        RT::on_next(521, 11),
        RT::on_next(561, 20),
        RT::on_completed::<i32>(601),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 600));
}

#[test]
fn replay_count_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = run_connectable(
        &scheduler,
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_buffer_on(3, scheduler)
        },
        450,
        475,
        &[(300, 400), (500, 550), (650, 800)],
    );

    assert_messages!(
        res.messages(),
        RT::on_next(451, 5),
        RT::on_next(452, 6),
        RT::on_next(453, 7),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 550), RT::subscribe(650, 800));
}

#[test]
fn replay_count_multiple_connections() {
    let xs = Observable::never::<i32>();
    let ys = xs.replay_buffer_on(3, TestScheduler::new().scheduler());

    let connection1 = ys.connect();
    let connection2 = ys.connect();

    assert!(std::ptr::addr_eq(Rc::as_ptr(&connection1), Rc::as_ptr(&connection2)));

    connection1.dispose();
    connection2.dispose();

    let connection3 = ys.connect();

    assert!(!std::ptr::addr_eq(Rc::as_ptr(&connection1), Rc::as_ptr(&connection3)));

    connection3.dispose();
}

#[test]
fn replay_count_lambda_zip_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = scheduler.start_disposed(
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_with_buffer_on(|xs| xs.take(6).repeat(), 3, scheduler)
        },
        610,
    );

    assert_messages!(
        res.messages(),
        RT::on_next(221, 3),
        RT::on_next(281, 4),
        RT::on_next(291, 1),
        RT::on_next(341, 8),
        RT::on_next(361, 5),
        RT::on_next(371, 6),
        RT::on_next(372, 8),
        RT::on_next(373, 5),
        RT::on_next(374, 6),
        RT::on_next(391, 7),
        RT::on_next(411, 13),
        RT::on_next(431, 2),
        RT::on_next(432, 7),
        RT::on_next(433, 13),
        RT::on_next(434, 2),
        RT::on_next(451, 9),
        RT::on_next(521, 11),
        RT::on_next(561, 20),
        RT::on_next(562, 9),
        RT::on_next(563, 11),
        RT::on_next(564, 20),
        RT::on_next(602, 9),
        RT::on_next(603, 11),
        RT::on_next(604, 20),
        RT::on_next(606, 9),
        RT::on_next(607, 11),
        RT::on_next(608, 20),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn replay_count_lambda_zip_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(source_messages(RT::on_error(600, ex.clone())));

    let res = scheduler.start({
        let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
        move || xs.as_observable().replay_with_buffer_on(|xs| xs.take(6).repeat(), 3, scheduler)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(221, 3),
        RT::on_next(281, 4),
        RT::on_next(291, 1),
        RT::on_next(341, 8),
        RT::on_next(361, 5),
        RT::on_next(371, 6),
        RT::on_next(372, 8),
        RT::on_next(373, 5),
        RT::on_next(374, 6),
        RT::on_next(391, 7),
        RT::on_next(411, 13),
        RT::on_next(431, 2),
        RT::on_next(432, 7),
        RT::on_next(433, 13),
        RT::on_next(434, 2),
        RT::on_next(451, 9),
        RT::on_next(521, 11),
        RT::on_next(561, 20),
        RT::on_next(562, 9),
        RT::on_next(563, 11),
        RT::on_next(564, 20),
        RT::on_error::<i32>(601, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn replay_count_lambda_zip_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = scheduler.start_disposed(
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_with_buffer_on(|xs| xs.take(6).repeat(), 3, scheduler)
        },
        470,
    );

    assert_messages!(
        res.messages(),
        RT::on_next(221, 3),
        RT::on_next(281, 4),
        RT::on_next(291, 1),
        RT::on_next(341, 8),
        RT::on_next(361, 5),
        RT::on_next(371, 6),
        RT::on_next(372, 8),
        RT::on_next(373, 5),
        RT::on_next(374, 6),
        RT::on_next(391, 7),
        RT::on_next(411, 13),
        RT::on_next(431, 2),
        RT::on_next(432, 7),
        RT::on_next(433, 13),
        RT::on_next(434, 2),
        RT::on_next(451, 9),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 470));
}

#[test]
fn replay_time_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = run_connectable(
        &scheduler,
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_window_on(ticks_to_duration(150), scheduler)
        },
        450,
        RT::DISPOSED,
        &[(300, 400), (500, 550), (650, 800)],
    );

    assert_messages!(
        res.messages(),
        RT::on_next(451, 8),
        RT::on_next(452, 5),
        RT::on_next(453, 6),
        RT::on_next(454, 7),
        RT::on_next(521, 11),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 550), RT::subscribe(650, 800));
}

#[test]
fn replay_time_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(source_messages(RT::on_error(600, ex.clone())));

    let res = run_connectable(
        &scheduler,
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_window_on(ticks_to_duration(75), scheduler)
        },
        450,
        RT::DISPOSED,
        &[(300, 400), (500, 800)],
    );

    assert_messages!(
        res.messages(),
        RT::on_next(451, 7),
        RT::on_next(521, 11),
        RT::on_next(561, 20),
        RT::on_error::<i32>(601, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 600));
}

#[test]
fn replay_time_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = run_connectable(
        &scheduler,
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_window_on(ticks_to_duration(85), scheduler)
        },
        450,
        RT::DISPOSED,
        &[(300, 400), (500, 800)],
    );

    assert_messages!(
        res.messages(),
        RT::on_next(451, 6),
        RT::on_next(452, 7),
        RT::on_next(521, 11),
        RT::on_next(561, 20),
        RT::on_completed::<i32>(601),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 600));
}

#[test]
fn replay_time_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = run_connectable(
        &scheduler,
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_window_on(ticks_to_duration(100), scheduler)
        },
        450,
        475,
        &[(300, 400), (500, 550), (650, 800)],
    );

    assert_messages!(
        res.messages(),
        RT::on_next(451, 5),
        RT::on_next(452, 6),
        RT::on_next(453, 7),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 550), RT::subscribe(650, 800));
}

#[test]
fn replay_time_multiple_connections() {
    let xs = Observable::never::<i32>();
    let ys = xs.replay_window_on(ticks_to_duration(100), TestScheduler::new().scheduler());

    let connection1 = ys.connect();
    let connection2 = ys.connect();

    assert!(std::ptr::addr_eq(Rc::as_ptr(&connection1), Rc::as_ptr(&connection2)));

    connection1.dispose();
    connection2.dispose();

    let connection3 = ys.connect();

    assert!(!std::ptr::addr_eq(Rc::as_ptr(&connection1), Rc::as_ptr(&connection3)));

    connection3.dispose();
}

#[test]
fn replay_time_lambda_zip_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = scheduler.start_disposed(
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_with_window_on(|xs| xs.take(6).repeat(), ticks_to_duration(50), scheduler)
        },
        610,
    );

    assert_messages!(
        res.messages(),
        RT::on_next(221, 3),
        RT::on_next(281, 4),
        RT::on_next(291, 1),
        RT::on_next(341, 8),
        RT::on_next(361, 5),
        RT::on_next(371, 6),
        RT::on_next(372, 8),
        RT::on_next(373, 5),
        RT::on_next(374, 6),
        RT::on_next(391, 7),
        RT::on_next(411, 13),
        RT::on_next(431, 2),
        RT::on_next(432, 7),
        RT::on_next(433, 13),
        RT::on_next(434, 2),
        RT::on_next(451, 9),
        RT::on_next(521, 11),
        RT::on_next(561, 20),
        RT::on_next(562, 11),
        RT::on_next(563, 20),
        RT::on_next(602, 20),
        RT::on_next(604, 20),
        RT::on_next(606, 20),
        RT::on_next(608, 20),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn replay_time_lambda_zip_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(source_messages(RT::on_error(600, ex.clone())));

    let res = scheduler.start({
        let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
        move || xs.as_observable().replay_with_window_on(|xs| xs.take(6).repeat(), ticks_to_duration(50), scheduler)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(221, 3),
        RT::on_next(281, 4),
        RT::on_next(291, 1),
        RT::on_next(341, 8),
        RT::on_next(361, 5),
        RT::on_next(371, 6),
        RT::on_next(372, 8),
        RT::on_next(373, 5),
        RT::on_next(374, 6),
        RT::on_next(391, 7),
        RT::on_next(411, 13),
        RT::on_next(431, 2),
        RT::on_next(432, 7),
        RT::on_next(433, 13),
        RT::on_next(434, 2),
        RT::on_next(451, 9),
        RT::on_next(521, 11),
        RT::on_next(561, 20),
        RT::on_next(562, 11),
        RT::on_next(563, 20),
        RT::on_error::<i32>(601, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn replay_time_lambda_zip_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = scheduler.start_disposed(
        {
            let (xs, scheduler) = (xs.clone(), scheduler.scheduler());
            move || xs.as_observable().replay_with_window_on(|xs| xs.take(6).repeat(), ticks_to_duration(50), scheduler)
        },
        470,
    );

    assert_messages!(
        res.messages(),
        RT::on_next(221, 3),
        RT::on_next(281, 4),
        RT::on_next(291, 1),
        RT::on_next(341, 8),
        RT::on_next(361, 5),
        RT::on_next(371, 6),
        RT::on_next(372, 8),
        RT::on_next(373, 5),
        RT::on_next(374, 6),
        RT::on_next(391, 7),
        RT::on_next(411, 13),
        RT::on_next(431, 2),
        RT::on_next(432, 7),
        RT::on_next(433, 13),
        RT::on_next(434, 2),
        RT::on_next(451, 9),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 470));
}

#[test]
fn replay_default1() {
    let s = Subject::<i32>::new();
    let xs = s.as_observable().replay_buffer_on(100, SchedulerDefaults::default_scheduler());
    let ys = s.as_observable().replay_buffer(100);

    xs.connect();
    ys.connect();

    s.on_next(1);
    s.on_next(2);
    s.on_completed();

    assert_eq!(notifications(&ys), notifications(&xs));
}

#[test]
fn replay_default2() {
    let s = Subject::<i32>::new();
    let xs = s.as_observable().replay_window_on(Duration::from_secs(3600), SchedulerDefaults::default_scheduler());
    let ys = s.as_observable().replay_window(Duration::from_secs(3600));

    xs.connect();
    ys.connect();

    s.on_next(1);
    s.on_next(2);
    s.on_completed();

    assert_eq!(notifications(&ys), notifications(&xs));
}

#[test]
fn replay_default3() {
    let s = Subject::<i32>::new();
    let xs = s.as_observable().replay_buffer_window_on(100, Duration::from_secs(3600), SchedulerDefaults::default_scheduler());
    let ys = s.as_observable().replay_buffer_window(100, Duration::from_secs(3600));

    xs.connect();
    ys.connect();

    s.on_next(1);
    s.on_next(2);
    s.on_completed();

    assert_eq!(notifications(&ys), notifications(&xs));
}

#[test]
fn replay_default4() {
    let s = Subject::<i32>::new();
    let xs = s.as_observable().replay_on(SchedulerDefaults::default_scheduler());
    let ys = s.as_observable().replay();

    xs.connect();
    ys.connect();

    s.on_next(1);
    s.on_next(2);
    s.on_completed();

    assert_eq!(notifications(&ys), notifications(&xs));
}

#[test]
fn replay_lambda_default1() {
    let xs = Observable::range(1, 10).replay_with_buffer_on(|xs| xs, 100, SchedulerDefaults::default_scheduler());
    let ys = Observable::range(1, 10).replay_with_buffer(|xs| xs, 100);

    assert_eq!(notifications(&ys), notifications(&xs));
}

#[test]
fn replay_lambda_default2() {
    let xs = Observable::range(1, 10).replay_with_window_on(|xs| xs, Duration::from_secs(3600), SchedulerDefaults::default_scheduler());
    let ys = Observable::range(1, 10).replay_with_window(|xs| xs, Duration::from_secs(3600));

    assert_eq!(notifications(&ys), notifications(&xs));
}

#[test]
fn replay_lambda_default3() {
    let xs = Observable::range(1, 10).replay_with_buffer_window_on(|xs| xs, 100, Duration::from_secs(3600), SchedulerDefaults::default_scheduler());
    let ys = Observable::range(1, 10).replay_with_buffer_window(|xs| xs, 100, Duration::from_secs(3600));

    assert_eq!(notifications(&ys), notifications(&xs));
}

#[test]
fn replay_lambda_default4() {
    let xs = Observable::range(1, 10).replay_with_on(|xs| xs, SchedulerDefaults::default_scheduler());
    let ys = Observable::range(1, 10).replay_with(|xs| xs);

    assert_eq!(notifications(&ys), notifications(&xs));
}

// Not ported: Replay_ArgumentChecking - argument validation (the negative buffer sizes and windows of the ArgumentOutOfRangeException cases cannot be expressed with usize and Duration)
