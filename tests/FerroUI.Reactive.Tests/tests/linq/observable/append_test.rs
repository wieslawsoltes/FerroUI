//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AppendTest.cs` of Rx.NET.
//! Adapted: `Append(value, scheduler)` is `append_on`; the sequence of nullable strings of
//! `Append_Null` is a sequence of `Option<&str>`, `null` being `None`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, ReactiveTest as RT, TestScheduler};

#[test]
fn append() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().append(3));

    assert_messages!(
        res.messages(),
        RT::on_next(220, 2),
        RT::on_next(250, 3),
        RT::on_completed(250),
    );
}

#[test]
fn append_null() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, Some("1")), RT::on_next(220, Some("2")), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().append(None));

    assert_messages!(
        res.messages(),
        RT::on_next(220, Some("2")),
        RT::on_next(250, None),
        RT::on_completed(250),
    );
}

#[test]
fn append_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().append_on(3, s));

    assert_messages!(
        res.messages(),
        RT::on_next(220, 2),
        RT::on_next(251, 3),
        RT::on_completed(251),
    );
}

#[test]
fn append_many() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().append(3).append(4).append(5));

    assert_messages!(
        res.messages(),
        RT::on_next(220, 2),
        RT::on_next(250, 3),
        RT::on_next(250, 4),
        RT::on_next(250, 5),
        RT::on_completed(250),
    );
}

#[test]
fn append_many_take() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().append(3).append(4).append(5).take(2));

    assert_messages!(
        res.messages(),
        RT::on_next(220, 2),
        RT::on_next(250, 3),
        RT::on_completed(250),
    );
}

#[test]
fn append_many_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().append_on(3, s.clone()).append_on(4, s.clone()).append_on(5, s));

    assert_messages!(
        res.messages(),
        RT::on_next(220, 2),
        RT::on_next(251, 3),
        RT::on_next(252, 4),
        RT::on_next(253, 5),
        RT::on_completed(253),
    );
}

#[test]
fn append_many_take_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().append_on(3, s.clone()).append_on(4, s.clone()).append_on(5, s).take(2));

    assert_messages!(
        res.messages(),
        RT::on_next(220, 2),
        RT::on_next(251, 3),
        RT::on_completed(251),
    );
}

// Not ported: Append_ArgumentChecking - argument validation
