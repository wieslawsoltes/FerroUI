//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/PublishLastTest.cs` of Rx.NET.
//! Adapted: the variables the scheduled actions assign are shared slots, set up by
//! `run_connectable` in the order of the upstream tests.

use ferroui_reactive::subjects::IConnectableObservable;
use ferroui_reactive::{IDisposable, IObservable, Notification, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, MockObserver, ReactiveTest as RT, Recorded, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;

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

#[test]
fn publish_last_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = run_connectable(
        &scheduler,
        {
            let xs = xs.clone();
            move || xs.as_observable().publish_last()
        },
        RT::SUBSCRIBED,
        RT::DISPOSED,
        &[(300, 400), (500, 550), (650, 800)],
    );

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 550), RT::subscribe(650, 800));
}

#[test]
fn publish_last_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(source_messages(RT::on_error(600, ex.clone())));

    let res = run_connectable(
        &scheduler,
        {
            let xs = xs.clone();
            move || xs.as_observable().publish_last()
        },
        RT::SUBSCRIBED,
        RT::DISPOSED,
        &[(300, 400), (500, 800)],
    );

    assert_messages!(res.messages(), RT::on_error::<i32>(600, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 600));
}

#[test]
fn publish_last_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = run_connectable(
        &scheduler,
        {
            let xs = xs.clone();
            move || xs.as_observable().publish_last()
        },
        RT::SUBSCRIBED,
        RT::DISPOSED,
        &[(300, 400), (500, 800)],
    );

    assert_messages!(res.messages(), RT::on_next(600, 20), RT::on_completed::<i32>(600));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 600));
}

#[test]
fn publish_last_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let res = run_connectable(
        &scheduler,
        {
            let xs = xs.clone();
            move || xs.as_observable().publish_last()
        },
        RT::SUBSCRIBED,
        350,
        &[(300, 400), (500, 550), (650, 800)],
    );

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(300, 400), RT::subscribe(500, 550), RT::subscribe(650, 800));
}

#[test]
fn publish_last_multiple_connections() {
    let xs = Observable::never::<i32>();
    let ys = xs.publish_last();

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
fn publish_last_lambda_zip_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.publish_last_with(|ys| ys.zip(ys.clone(), |x, y| x + y)));

    assert_messages!(res.messages(), RT::on_next(600, 40), RT::on_completed(600));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn publish_last_lambda_zip_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(source_messages(RT::on_error(600, ex.clone())));

    let source = xs.as_observable();
    let res = scheduler.start(move || source.publish_last_with(|ys| ys.zip(ys.clone(), |x, y| x + y)));

    assert_messages!(res.messages(), RT::on_error(600, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn publish_last_lambda_zip_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(source_messages(RT::on_completed(600)));

    let source = xs.as_observable();
    let res = scheduler.start_disposed(move || source.publish_last_with(|ys| ys.zip(ys.clone(), |x, y| x + y)), 470);

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 470));
}

// Not ported: PublishLast_ArgumentChecking - argument validation
