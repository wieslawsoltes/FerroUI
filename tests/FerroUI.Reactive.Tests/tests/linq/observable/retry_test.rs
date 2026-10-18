//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RetryTest.cs` of Rx.NET.
//! Adapted: `Retry(int)` is `retry_count`; an exception thrown by an observer is a panic, checked
//! with `assert_throws`.

use ferroui_reactive::{IDisposable, IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, assert_throws, new_error, ReactiveTest as RT, TestScheduler};
use std::rc::Rc;

#[test]
fn retry_observable_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_completed(250)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().retry()
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_completed(450));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn retry_observable_infinite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().retry()
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn retry_observable_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_error(250, ex.clone())]);

    let res = scheduler.start_disposed(
        {
            let xs = xs.clone();
            move || xs.as_observable().retry()
        },
        1100,
    );

    assert_messages!(
        res.messages(),
        RT::on_next(300, 1),
        RT::on_next(350, 2),
        RT::on_next(400, 3),
        RT::on_next(550, 1),
        RT::on_next(600, 2),
        RT::on_next(650, 3),
        RT::on_next(800, 1),
        RT::on_next(850, 2),
        RT::on_next(900, 3),
        RT::on_next(1050, 1)
    );

    assert_subscriptions!(
        xs.subscriptions(),
        RT::subscribe(200, 450),
        RT::subscribe(450, 700),
        RT::subscribe(700, 950),
        RT::subscribe(950, 1100)
    );
}

#[test]
fn retry_observable_throws1() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::return_on(1, scheduler1.scheduler()).retry();

    let _d = xs.subscribe_next(|_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler1.run());
}

#[test]
fn retry_observable_throws2() {
    let scheduler2 = TestScheduler::new();

    let ys = Observable::throw_on::<i32>(new_error("ex"), scheduler2.scheduler()).retry();

    let d = ys.subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));

    scheduler2.schedule_absolute_action(210, move || d.dispose());

    scheduler2.run();
}

#[test]
fn retry_observable_throws3() {
    let scheduler3 = TestScheduler::new();

    let zs = Observable::return_on(1, scheduler3.scheduler()).retry();

    let _d = zs.subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));

    assert_throws(|| scheduler3.run());
}

#[test]
fn retry_observable_retry_count_basic() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(5, 1), RT::on_next(10, 2), RT::on_next(15, 3), RT::on_error(20, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().retry_count(3)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(205, 1),
        RT::on_next(210, 2),
        RT::on_next(215, 3),
        RT::on_next(225, 1),
        RT::on_next(230, 2),
        RT::on_next(235, 3),
        RT::on_next(245, 1),
        RT::on_next(250, 2),
        RT::on_next(255, 3),
        RT::on_error(260, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220), RT::subscribe(220, 240), RT::subscribe(240, 260));
}

#[test]
fn retry_observable_retry_count_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(5, 1), RT::on_next(10, 2), RT::on_next(15, 3), RT::on_error(20, new_error("ex"))]);

    let res = scheduler.start_disposed(
        {
            let xs = xs.clone();
            move || xs.as_observable().retry_count(3)
        },
        231,
    );

    assert_messages!(res.messages(), RT::on_next(205, 1), RT::on_next(210, 2), RT::on_next(215, 3), RT::on_next(225, 1), RT::on_next(230, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220), RT::subscribe(220, 231));
}

#[test]
fn retry_observable_retry_count_infinite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().retry_count(3)
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn retry_observable_retry_count_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_completed(250)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().retry_count(3)
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_completed(450));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn retry_observable_retry_count_throws() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::return_on(1, scheduler1.scheduler()).retry_count(3);

    let _d1 = xs.subscribe_next(|_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler1.run());

    let scheduler2 = TestScheduler::new();

    let ys = Observable::throw_on::<i32>(new_error("ex"), scheduler2.scheduler()).retry_count(100);

    let d = ys.subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));

    scheduler2.schedule_absolute_action(10, move || d.dispose());

    scheduler2.run();

    let scheduler3 = TestScheduler::new();

    let zs = Observable::return_on(1, scheduler3.scheduler()).retry_count(100);

    let _d3 = zs.subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));

    assert_throws(|| scheduler3.run());

    let xss = Observable::create(|_o: Rc<dyn IObserver<i32>>| -> Rc<dyn IDisposable> { panic!("InvalidOperationException") }).retry_count(3);

    assert_throws(|| {
        xss.subscribe_ignore();
    });
}

// Not ported: Retry_Observable_ArgumentChecking - argument validation
// Not ported: Retry_Observable_Default_ArgumentChecking - argument validation
// Not ported: Retry_Observable_RetryCount_ArgumentChecking - argument validation (the count is a `usize`, never negative)
// Not ported: Retry_Observable_RetryCount_Default_ArgumentChecking - argument validation (the count is a `usize`, never negative)
// Not ported: Retry_Observable_RetryCount_Default - blocking operator ToEnumerable (of the AssertEqual of two sequences) (waived: one thread)
