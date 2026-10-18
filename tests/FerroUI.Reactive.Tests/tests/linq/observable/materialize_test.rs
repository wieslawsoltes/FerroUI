//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/MaterializeTest.cs` of Rx.NET.

use ferroui_reactive::{Notification, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn materialize_never() {
    let scheduler = TestScheduler::new();
    let res = scheduler.start(|| Observable::never::<i32>().materialize());

    assert_messages!(res.messages());
}

#[test]
fn materialize_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().materialize());

    assert_messages!(res.messages(), RT::on_next(250, Notification::<i32>::OnCompleted), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn materialize_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().materialize());

    assert_messages!(
        res.messages(),
        RT::on_next(210, Notification::OnNext(2)),
        RT::on_next(250, Notification::<i32>::OnCompleted),
        RT::on_completed(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn materialize_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, ex.clone())]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().materialize());

    assert_messages!(res.messages(), RT::on_next(250, Notification::<i32>::OnError(ex.clone())), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

// Not ported: Materialize_ArgumentChecking - argument validation
