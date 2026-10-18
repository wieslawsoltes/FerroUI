//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/PrependTest.cs` of Rx.NET.
//! Adapted: `Prepend(value, scheduler)` is `prepend_on`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, ReactiveTest as RT, TestScheduler};

#[test]
fn prepend() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().prepend(3));

    assert_messages!(
        res.messages(),
        RT::on_next(200, 3),
        RT::on_next(220, 2),
        RT::on_completed(250),
    );
}

#[test]
fn prepend_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 4), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().prepend_on(3, s));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 3),
        RT::on_next(220, 4),
        RT::on_completed(250),
    );
}

#[test]
fn prepend_many() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().prepend(3).prepend(4).prepend(5));

    assert_messages!(
        res.messages(),
        RT::on_next(200, 5),
        RT::on_next(200, 4),
        RT::on_next(200, 3),
        RT::on_next(220, 2),
        RT::on_completed(250),
    );
}

#[test]
fn prepend_many_take() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().prepend(3).prepend(4).prepend(5).take(2));

    assert_messages!(
        res.messages(),
        RT::on_next(200, 5),
        RT::on_next(200, 4),
        RT::on_completed(200),
    );
}

#[test]
fn prepend_many_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().prepend_on(3, s.clone()).prepend_on(4, s.clone()).prepend_on(5, s));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 5),
        RT::on_next(202, 4),
        RT::on_next(203, 3),
        RT::on_next(220, 2),
        RT::on_completed(250),
    );
}

#[test]
fn prepend_many_take_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().prepend_on(3, s.clone()).prepend_on(4, s.clone()).prepend_on(5, s).take(2));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 5),
        RT::on_next(202, 4),
        RT::on_completed(202),
    );
}

// Not ported: Prepend_ArgumentChecking - argument validation
