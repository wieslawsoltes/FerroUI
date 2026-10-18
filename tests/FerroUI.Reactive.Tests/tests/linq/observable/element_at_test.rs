//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ElementAtTest.cs` of Rx.NET.
//! Adapted: the error of an index out of range (`e is ArgumentOutOfRangeException`) is recognized
//! by its message.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn element_at_first() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_next(470, 44), RT::on_completed(600)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().element_at(0)
    });

    assert_messages!(res.messages(), RT::on_next(280, 42), RT::on_completed(280));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 280));
}

#[test]
fn element_at_other() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_next(470, 44), RT::on_completed(600)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().element_at(2)
    });

    assert_messages!(res.messages(), RT::on_next(470, 44), RT::on_completed(470));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 470));
}

#[test]
fn element_at_out_of_range() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_next(470, 44), RT::on_completed(600)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().element_at(3)
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<i32>(600, |e| e.to_string() == "Specified argument was out of the range of valid values. (Parameter 'index')")
    );
}

#[test]
fn element_at_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 42), RT::on_next(360, 43), RT::on_error(420, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().element_at(3)
    });

    assert_messages!(res.messages(), RT::on_error(420, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

// Not ported: ElementAt_ArgumentChecking - argument validation (the index is a `usize`, never negative)
