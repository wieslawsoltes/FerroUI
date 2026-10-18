//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RepeatTest.cs` of Rx.NET: the forms of
//! a source (`Repeat<TSource>(IObservable<TSource>)` and `Repeat<TSource>(IObservable<TSource>,
//! int)`); the forms of a value are in `repeat_test.rs`.
//! Adapted: `Repeat(int)` of a source is `repeat_count`; an exception thrown by an observer is a
//! panic, checked with `assert_throws`.

use ferroui_reactive::{IDisposable, IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, assert_throws, new_error, ReactiveTest as RT, TestScheduler};
use std::rc::Rc;

#[test]
fn repeat_observable_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_completed(250)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat()
    });

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
        RT::on_next(900, 3)
    );

    assert_subscriptions!(
        xs.subscriptions(),
        RT::subscribe(200, 450),
        RT::subscribe(450, 700),
        RT::subscribe(700, 950),
        RT::subscribe(950, 1000)
    );
}

#[test]
fn repeat_observable_infinite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat()
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn repeat_observable_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_error(250, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat()
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_error(450, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn repeat_observable_throws() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::return_on(1, scheduler1.scheduler()).repeat();

    let _d1 = xs.subscribe_next(|_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler1.run());

    let scheduler2 = TestScheduler::new();

    let ys = Observable::throw_on::<i32>(new_error("ex"), scheduler2.scheduler()).repeat();

    let _d2 = ys.subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler2.run());

    let scheduler3 = TestScheduler::new();

    let zs = Observable::return_on(1, scheduler3.scheduler()).repeat();

    let d = zs.subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));

    scheduler3.schedule_absolute_action(210, move || d.dispose());

    scheduler3.run();

    let xss = Observable::create(|_o: Rc<dyn IObserver<i32>>| -> Rc<dyn IDisposable> { panic!("InvalidOperationException") }).repeat();

    assert_throws(|| {
        xss.subscribe_ignore();
    });
}

#[test]
fn repeat_observable_repeat_count_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(5, 1), RT::on_next(10, 2), RT::on_next(15, 3), RT::on_completed(20)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_count(3)
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
        RT::on_completed(260)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220), RT::subscribe(220, 240), RT::subscribe(240, 260));
}

#[test]
fn repeat_observable_repeat_count_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(5, 1), RT::on_next(10, 2), RT::on_next(15, 3), RT::on_completed(20)]);

    let res = scheduler.start_disposed(
        {
            let xs = xs.clone();
            move || xs.as_observable().repeat_count(3)
        },
        231,
    );

    assert_messages!(res.messages(), RT::on_next(205, 1), RT::on_next(210, 2), RT::on_next(215, 3), RT::on_next(225, 1), RT::on_next(230, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220), RT::subscribe(220, 231));
}

#[test]
fn repeat_observable_repeat_count_infinite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_count(3)
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn repeat_observable_repeat_count_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_error(250, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_count(3)
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_error(450, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn repeat_observable_repeat_count_throws() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::return_on(1, scheduler1.scheduler()).repeat_count(3);

    let _d1 = xs.subscribe_next(|_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler1.run());

    let scheduler2 = TestScheduler::new();

    let ys = Observable::throw_on::<i32>(new_error("ex"), scheduler2.scheduler()).repeat_count(3);

    let _d2 = ys.subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler2.run());

    let scheduler3 = TestScheduler::new();

    let zs = Observable::return_on(1, scheduler3.scheduler()).repeat_count(100);

    let d = zs.subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));

    scheduler3.schedule_absolute_action(10, move || d.dispose());

    scheduler3.run();

    let xss = Observable::create(|_o: Rc<dyn IObserver<i32>>| -> Rc<dyn IDisposable> { panic!("InvalidOperationException") }).repeat_count(3);

    assert_throws(|| {
        xss.subscribe_ignore();
    });
}

// Not ported: Repeat_Observable_ArgumentChecking - argument validation
// Not ported: Repeat_Observable_Default_ArgumentChecking - argument validation
// Not ported: Repeat_Observable_RepeatCount_ArgumentChecking - argument validation (the count is a `usize`, never negative)
// Not ported: Repeat_Observable_RepeatCount_Default_ArgumentChecking - argument validation (the count is a `usize`, never negative)
