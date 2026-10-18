//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/StartWithTest.cs` of Rx.NET.
//! Adapted: `StartWith(params T[])` and `StartWith(IEnumerable<T>)` are both `start_with` of an
//! iterable, and their overloads with a scheduler are `start_with_on`, whose scheduler is the last
//! argument.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, ReactiveTest as RT, TestScheduler};

#[test]
fn start_with() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().start_with([1]));

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(220, 2), RT::on_completed(250));
}

#[test]
fn start_with_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 4), RT::on_completed(250)]);

    let on = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().start_with_on([1, 2, 3], on));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 1),
        RT::on_next(202, 2),
        RT::on_next(203, 3),
        RT::on_next(220, 4),
        RT::on_completed(250),
    );
}

#[test]
fn start_with_enumerable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 4), RT::on_completed(250)]);

    let data: Vec<i32> = vec![1, 2, 3];
    let res = scheduler.start(move || xs.as_observable().start_with(data));

    assert_messages!(
        res.messages(),
        RT::on_next(200, 1),
        RT::on_next(200, 2),
        RT::on_next(200, 3),
        RT::on_next(220, 4),
        RT::on_completed(250),
    );
}

#[test]
fn start_with_enumerable_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 4), RT::on_completed(250)]);

    let data: Vec<i32> = vec![1, 2, 3];
    let on = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().start_with_on(data, on));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 1),
        RT::on_next(202, 2),
        RT::on_next(203, 3),
        RT::on_next(220, 4),
        RT::on_completed(250),
    );
}

// Not ported: StartWith_ArgumentChecking - argument validation
