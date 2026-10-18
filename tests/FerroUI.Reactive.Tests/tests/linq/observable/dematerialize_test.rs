//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DematerializeTest.cs` of Rx.NET.

use ferroui_reactive::{Notification, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn dematerialize_range1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Notification::OnNext(41)),
        RT::on_next(210, Notification::OnNext(42)),
        RT::on_next(220, Notification::OnNext(43)),
        RT::on_completed(250),
    ]);
    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().dematerialize());

    assert_messages!(res.messages(), RT::on_next(210, 42), RT::on_next(220, 43), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn dematerialize_range2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Notification::OnNext(41)),
        RT::on_next(210, Notification::OnNext(42)),
        RT::on_next(220, Notification::OnNext(43)),
        RT::on_next(230, Notification::OnCompleted),
    ]);
    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().dematerialize());

    assert_messages!(res.messages(), RT::on_next(210, 42), RT::on_next(220, 43), RT::on_completed(230));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn dematerialize_error1() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Notification::OnNext(41)),
        RT::on_next(210, Notification::OnNext(42)),
        RT::on_next(220, Notification::OnNext(43)),
        RT::on_error(230, ex.clone()),
    ]);
    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().dematerialize());

    assert_messages!(res.messages(), RT::on_next(210, 42), RT::on_next(220, 43), RT::on_error(230, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn dematerialize_error2() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Notification::OnNext(41)),
        RT::on_next(210, Notification::OnNext(42)),
        RT::on_next(220, Notification::OnNext(43)),
        RT::on_next(230, Notification::OnError(ex.clone())),
    ]);
    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().dematerialize());

    assert_messages!(res.messages(), RT::on_next(210, 42), RT::on_next(220, 43), RT::on_error(230, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn materialize_dematerialize_never() {
    let scheduler = TestScheduler::new();

    let xs = Observable::never::<i32>();

    let res = scheduler.start(move || xs.materialize().dematerialize());

    assert_messages!(res.messages());
}

#[test]
fn materialize_dematerialize_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().materialize().dematerialize());

    assert_messages!(res.messages(), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn materialize_dematerialize_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().materialize().dematerialize());

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn materialize_dematerialize_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, ex.clone())]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().materialize().dematerialize());

    assert_messages!(res.messages(), RT::on_error(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

// Not ported: Dematerialize_ArgumentChecking - argument validation
