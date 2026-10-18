//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/LastAsyncTest.cs` of Rx.NET.
//! Adapted: the overload with a predicate is `last_async_with`; a predicate that throws is
//! `try_last_async_with` returning `Err(ex)`; `InvalidOperationException` is `SequenceError`.

use ferroui_reactive::linq::SequenceError;
use ferroui_reactive::{ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

/// `e is InvalidOperationException`.
fn is_invalid_operation(error: &ObservableError) -> bool {
    error.downcast_ref::<SequenceError>().is_some_and(SequenceError::is_invalid_operation)
}

#[test]
fn last_async_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().last_async()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<i32>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn last_async_one() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().last_async()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn last_async_many() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().last_async()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn last_async_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().last_async()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn last_async_predicate() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().last_async_with(|x| x % 2 == 1)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 5),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn last_async_predicate_none() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().last_async_with(|x| *x > 10)
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<i32>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn last_async_predicate_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().last_async_with(|x| x % 2 == 1)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn last_async_predicate_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_last_async_with(move |x| if *x < 4 { Ok(x % 2 == 1) } else { Err(ex.clone()) })
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(230, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 230));
}

// Not ported: LastAsync_ArgumentChecking - argument validation
