//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/IfTest.cs` of Rx.NET.
//! Adapted: `If(condition, then, else)` is `Observable::if_`, `If(condition, then)` is
//! `Observable::if_then` and `If(condition, then, scheduler)` is `Observable::if_then_on`; a
//! condition that throws is the `try_if` form returning the error.

use ferroui_reactive::Observable;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn if_true() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(250, 2), RT::on_completed(300)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(310, 3), RT::on_next(350, 4), RT::on_completed(400)]);

    let (x, y) = (xs.as_observable(), ys.as_observable());
    let results = scheduler.start(move || Observable::if_(|| true, x, y));

    assert_messages!(results.messages(), RT::on_next(210, 1), RT::on_next(250, 2), RT::on_completed(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn if_false() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(250, 2), RT::on_completed(300)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(310, 3), RT::on_next(350, 4), RT::on_completed(400)]);

    let (x, y) = (xs.as_observable(), ys.as_observable());
    let results = scheduler.start(move || Observable::if_(|| false, x, y));

    assert_messages!(results.messages(), RT::on_next(310, 3), RT::on_next(350, 4), RT::on_completed(400));

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn if_throw() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(250, 2), RT::on_completed(300)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(310, 3), RT::on_next(350, 4), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (x, y, e) = (xs.as_observable(), ys.as_observable(), ex.clone());
    let results = scheduler.start(move || Observable::try_if(move || Err(e.clone()), x, y));

    assert_messages!(results.messages(), RT::on_error::<i32>(200, ex));

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn if_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(250, 2)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(310, 3), RT::on_next(350, 4), RT::on_completed(400)]);

    let (x, y) = (xs.as_observable(), ys.as_observable());
    let results = scheduler.start(move || Observable::if_(|| true, x, y));

    assert_messages!(results.messages(), RT::on_next(210, 1), RT::on_next(250, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn if_default_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(110, 1), RT::on_next(220, 2), RT::on_next(330, 3), RT::on_completed(440)]);

    let b = Rc::new(Cell::new(false));

    let b1 = b.clone();
    scheduler.schedule_absolute_action(150, move || b1.set(true));

    let x = xs.as_observable();
    let results = scheduler.start(move || Observable::if_then(move || b.get(), x));

    assert_messages!(results.messages(), RT::on_next(220, 2), RT::on_next(330, 3), RT::on_completed(440));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 440));
}

#[test]
fn if_default_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(110, 1), RT::on_next(220, 2), RT::on_next(330, 3), RT::on_error(440, ex.clone())]);

    let b = Rc::new(Cell::new(false));

    let b1 = b.clone();
    scheduler.schedule_absolute_action(150, move || b1.set(true));

    let x = xs.as_observable();
    let results = scheduler.start(move || Observable::if_then(move || b.get(), x));

    assert_messages!(results.messages(), RT::on_next(220, 2), RT::on_next(330, 3), RT::on_error(440, ex));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 440));
}

#[test]
fn if_default_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(110, 1), RT::on_next(220, 2), RT::on_next(330, 3)]);

    let b = Rc::new(Cell::new(false));

    let b1 = b.clone();
    scheduler.schedule_absolute_action(150, move || b1.set(true));

    let x = xs.as_observable();
    let results = scheduler.start(move || Observable::if_then(move || b.get(), x));

    assert_messages!(results.messages(), RT::on_next(220, 2), RT::on_next(330, 3));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn if_default_other() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(110, 1), RT::on_next(220, 2), RT::on_next(330, 3), RT::on_error(440, new_error("ex"))]);

    let b = Rc::new(Cell::new(true));

    let b1 = b.clone();
    scheduler.schedule_absolute_action(150, move || b1.set(false));

    let x = xs.as_observable();
    let results = scheduler.start(move || Observable::if_then(move || b.get(), x));

    assert_messages!(results.messages(), RT::on_completed::<i32>(200));

    assert_subscriptions!(xs.subscriptions());
}

#[test]
fn if_default_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(110, 1), RT::on_next(220, 2), RT::on_next(330, 3), RT::on_error(440, new_error("ex"))]);

    let (x, s) = (xs.as_observable(), scheduler.scheduler());
    let results = scheduler.start(move || Observable::if_then_on(|| false, x, s));

    assert_messages!(results.messages(), RT::on_completed::<i32>(201));

    assert_subscriptions!(xs.subscriptions());
}

// Not ported: If_ArgumentChecking - argument validation
// Not ported: If_Default_ArgumentChecking - argument validation
