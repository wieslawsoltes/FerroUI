//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/IgnoreElementsTest.cs` of Rx.NET.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn ignore_elements_ignore_elements() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_completed(300),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().ignore_elements().ignore_elements());

    assert_messages!(res.messages(), RT::on_completed(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn ignore_elements_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().ignore_elements());

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn ignore_elements_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_completed(610),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().ignore_elements());

    assert_messages!(res.messages(), RT::on_completed(610));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 610));
}

#[test]
fn ignore_elements_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_error(610, ex.clone()),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().ignore_elements());

    assert_messages!(res.messages(), RT::on_error(610, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 610));
}

// Not ported: IgnoreElements_ArgumentChecking - argument validation
