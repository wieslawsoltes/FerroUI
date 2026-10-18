//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DefaultIfEmptyTest.cs` of Rx.NET.
//! Adapted: `DefaultIfEmpty(defaultValue)` is `default_if_empty_with`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn default_if_empty_non_empty1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_completed(420)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().default_if_empty());

    assert_messages!(res.messages(), RT::on_next(280, 42), RT::on_next(360, 43), RT::on_completed(420));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn default_if_empty_non_empty2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_completed(420)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().default_if_empty_with(-1));

    assert_messages!(res.messages(), RT::on_next(280, 42), RT::on_next(360, 43), RT::on_completed(420));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn default_if_empty_empty1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(420)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().default_if_empty());

    assert_messages!(res.messages(), RT::on_next(420, 0), RT::on_completed(420));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn default_if_empty_empty2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(420)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().default_if_empty_with(-1));

    assert_messages!(res.messages(), RT::on_next(420, -1), RT::on_completed(420));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn default_if_empty_throw1() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_error::<i32>(420, ex.clone())]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().default_if_empty());

    assert_messages!(res.messages(), RT::on_error(420, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn default_if_empty_throw2() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_error::<i32>(420, ex.clone())]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().default_if_empty_with(42));

    assert_messages!(res.messages(), RT::on_error(420, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

// Not ported: DefaultIfEmpty_ArgumentChecking - argument validation
