//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DistinctUntilChangedTest.cs` of Rx.NET.
//! Adapted: an `IEqualityComparer<T>` (`FuncComparer`) is a function of equality; a key selector or
//! a comparer that throws (`ThrowComparer`) is the `try_` form returning the error.

use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn distinct_until_changed_never() {
    let scheduler = TestScheduler::new();

    let xs = Observable::never::<i32>();

    let res = scheduler.start(move || xs.distinct_until_changed());

    assert_messages!(res.messages());
}

#[test]
fn distinct_until_changed_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed()
    });

    assert_messages!(res.messages(), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn distinct_until_changed_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed()
    });

    assert_messages!(res.messages(), RT::on_next(220, 2), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn distinct_until_changed_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");
    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed()
    });

    assert_messages!(res.messages(), RT::on_error(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn distinct_until_changed_all_changes() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn distinct_until_changed_all_same() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 2),
        RT::on_next(230, 2),
        RT::on_next(240, 2),
        RT::on_completed(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed()
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn distinct_until_changed_some_changes() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2), //*
        RT::on_next(215, 3), //*
        RT::on_next(220, 3),
        RT::on_next(225, 2), //*
        RT::on_next(230, 2),
        RT::on_next(230, 1), //*
        RT::on_next(240, 2), //*
        RT::on_completed(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(215, 3),
        RT::on_next(225, 2),
        RT::on_next(230, 1),
        RT::on_next(240, 2),
        RT::on_completed(250)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn distinct_until_changed_comparer_all_equal() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed_with_comparer(|_, _| true)
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn distinct_until_changed_comparer_all_different() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 2),
        RT::on_next(230, 2),
        RT::on_next(240, 2),
        RT::on_completed(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed_with_comparer(|_, _| false)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 2),
        RT::on_next(230, 2),
        RT::on_next(240, 2),
        RT::on_completed(250)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn distinct_until_changed_key_selector_div2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2), //*
        RT::on_next(220, 4),
        RT::on_next(230, 3), //*
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed_by_key(|x| x % 2)
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(230, 3), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn distinct_until_changed_key_selector_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let res = scheduler.start({
        let (xs, ex) = (xs.clone(), ex.clone());
        move || xs.as_observable().try_distinct_until_changed_by_key(move |_| Err::<i32, _>(ex.clone()))
    });

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));
}

#[test]
fn distinct_until_changed_comparer_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_completed(250)]);

    let res = scheduler.start({
        let (xs, ex) = (xs.clone(), ex.clone());
        move || xs.as_observable().try_distinct_until_changed_with_comparer(move |_, _| Err(ex.clone()))
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_error(220, ex.clone()));
}

#[test]
fn distinct_until_changed_key_selector_comparer() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2), // * key = 1    % 3 = 1
        RT::on_next(220, 8), //   key = 4    % 3 = 1   same
        RT::on_next(230, 2), //   key = 1    % 3 = 1   same
        RT::on_next(240, 5), // * key = 2    % 3 = 2
        RT::on_completed(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_until_changed_with(|x| x / 2, |x: &i32, y: &i32| x % 3 == y % 3)
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(240, 5), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

// Not ported: DistinctUntilChanged_ArgumentChecking - argument validation
