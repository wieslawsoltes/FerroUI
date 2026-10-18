//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ContainsTest.cs` of Rx.NET.
//! Adapted: the comparer classes are closures of `contains_with`; the comparer that throws
//! `NotImplementedException` is `try_contains_with` returning an error of a local type.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::rc::Rc;

/// `NotImplementedException`, which the comparer of `ContainsComparerThrows` throws.
#[derive(Debug)]
struct NotImplementedError;

impl std::fmt::Display for NotImplementedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The method or operation is not implemented.")
    }
}

impl std::error::Error for NotImplementedError {}

/// The comparer of `ContainsComparerMod2`: equal when the parity is.
fn mod2(x: &i32, y: &i32) -> bool {
    x % 2 == y % 2
}

#[test]
fn contains_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().contains(42)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, false),
        RT::on_completed::<bool>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn contains_return_positive() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().contains(2)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, true),
        RT::on_completed::<bool>(210),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn contains_return_negative() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().contains(-2)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, false),
        RT::on_completed::<bool>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn contains_some_positive() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().contains(3)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, true),
        RT::on_completed::<bool>(220),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn contains_some_negative() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().contains(-3)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, false),
        RT::on_completed::<bool>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn contains_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().contains(42)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn contains_never() {
    let _ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().contains(42)
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn contains_comparer_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().try_contains_with(42, |_, _| Err(Rc::new(NotImplementedError)))
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<bool>(210, |e| e.downcast_ref::<NotImplementedError>().is_some()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn contains_comparer_contains_value() {
    let _ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 3),
        RT::on_next(220, 4),
        RT::on_next(230, 8),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().contains_with(42, mod2)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, true),
        RT::on_completed::<bool>(220),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn contains_comparer_does_not_contain_value() {
    let _ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 4),
        RT::on_next(230, 8),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().contains_with(21, mod2)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, false),
        RT::on_completed::<bool>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

// Not ported: Contains_ArgumentChecking - argument validation
