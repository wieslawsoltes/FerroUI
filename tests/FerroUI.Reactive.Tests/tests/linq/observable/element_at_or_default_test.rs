//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ElementAtOrDefaultTest.cs` of Rx.NET.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn element_at_or_default_first() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_next(470, 44), RT::on_completed(600)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().element_at_or_default(0)
    });

    assert_messages!(res.messages(), RT::on_next(280, 42), RT::on_completed(280));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 280));
}

#[test]
fn element_at_or_default_other() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_next(470, 44), RT::on_completed(600)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().element_at_or_default(2)
    });

    assert_messages!(res.messages(), RT::on_next(470, 44), RT::on_completed(470));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 470));
}

#[test]
fn element_at_or_default_out_of_range() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_next(470, 44), RT::on_completed(600)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().element_at_or_default(3)
    });

    assert_messages!(res.messages(), RT::on_next(600, 0), RT::on_completed(600));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

#[test]
fn element_at_or_default_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_error(420, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().element_at_or_default(3)
    });

    assert_messages!(res.messages(), RT::on_error(420, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

// Not ported: ElementAtOrDefault_ArgumentChecking - argument validation (the index is a `usize`, never negative)
