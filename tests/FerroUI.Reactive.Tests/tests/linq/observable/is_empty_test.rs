//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/IsEmptyTest.cs` of Rx.NET.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn is_empty_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().is_empty()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, true),
        RT::on_completed::<bool>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn is_empty_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().is_empty()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, false),
        RT::on_completed::<bool>(210),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn is_empty_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().is_empty()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn is_empty_never() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().is_empty()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

// Not ported: IsEmpty_ArgumentChecking - argument validation
