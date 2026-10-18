//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/WithLatestFromTest.cs` of Rx.NET.
//! Adapted: the strings of the second sequence are `&'static str`, the results `String`; the selector
//! that throws is the selector of `try_with_latest_from` returning the error.

use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

fn s(value: &str) -> String {
    value.to_owned()
}

#[test]
fn with_latest_from_simple1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(180, 2),
        RT::on_next(250, 3),
        RT::on_next(260, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(410, 7),
        RT::on_next(420, 8),
        RT::on_next(470, 9),
        RT::on_next(550, 10),
        RT::on_completed(590),
    ]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(255, "bar"), RT::on_next(330, "foo"), RT::on_next(350, "qux"), RT::on_completed(400)]);

    let (a, b) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || a.with_latest_from(b, |x, y| format!("{x}{y}")));

    assert_messages!(
        res.messages(),
        RT::on_next(260, s("4bar")),
        RT::on_next(310, s("5bar")),
        RT::on_next(340, s("6foo")),
        RT::on_next(410, s("7qux")),
        RT::on_next(420, s("8qux")),
        RT::on_next(470, s("9qux")),
        RT::on_next(550, s("10qux")),
        RT::on_completed(590)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn with_latest_from_simple2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(180, 2),
        RT::on_next(250, 3),
        RT::on_next(260, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_completed(390),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(255, "bar"),
        RT::on_next(330, "foo"),
        RT::on_next(350, "qux"),
        RT::on_next(370, "baz"),
        RT::on_completed(400),
    ]);

    let (a, b) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || a.with_latest_from(b, |x, y| format!("{x}{y}")));

    assert_messages!(res.messages(), RT::on_next(260, s("4bar")), RT::on_next(310, s("5bar")), RT::on_next(340, s("6foo")), RT::on_completed(390));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 390));
}

#[test]
fn with_latest_from_simple3() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(180, 2),
        RT::on_next(250, 3),
        RT::on_next(260, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_completed(390),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(245, "bar"),
        RT::on_next(330, "foo"),
        RT::on_next(350, "qux"),
        RT::on_next(370, "baz"),
        RT::on_completed(400),
    ]);

    let (a, b) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || a.with_latest_from(b, |x, y| format!("{x}{y}")));

    assert_messages!(
        res.messages(),
        RT::on_next(250, s("3bar")),
        RT::on_next(260, s("4bar")),
        RT::on_next(310, s("5bar")),
        RT::on_next(340, s("6foo")),
        RT::on_completed(390)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 390));
}

#[test]
fn with_latest_from_error1() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(180, 2),
        RT::on_next(250, 3),
        RT::on_next(260, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(410, 7),
        RT::on_next(420, 8),
        RT::on_next(470, 9),
        RT::on_next(550, 10),
        RT::on_error(590, ex.clone()),
    ]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(255, "bar"), RT::on_next(330, "foo"), RT::on_next(350, "qux"), RT::on_completed(400)]);

    let (a, b) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || a.with_latest_from(b, |x, y| format!("{x}{y}")));

    assert_messages!(
        res.messages(),
        RT::on_next(260, s("4bar")),
        RT::on_next(310, s("5bar")),
        RT::on_next(340, s("6foo")),
        RT::on_next(410, s("7qux")),
        RT::on_next(420, s("8qux")),
        RT::on_next(470, s("9qux")),
        RT::on_next(550, s("10qux")),
        RT::on_error(590, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 590));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn with_latest_from_error2() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(180, 2),
        RT::on_next(250, 3),
        RT::on_next(260, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_completed(390),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(255, "bar"),
        RT::on_next(330, "foo"),
        RT::on_next(350, "qux"),
        RT::on_error(370, ex.clone()),
    ]);

    let (a, b) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || a.with_latest_from(b, |x, y| format!("{x}{y}")));

    assert_messages!(
        res.messages(),
        RT::on_next(260, s("4bar")),
        RT::on_next(310, s("5bar")),
        RT::on_next(340, s("6foo")),
        RT::on_error(370, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 370));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 370));
}

#[test]
fn with_latest_from_error3() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(180, 2),
        RT::on_next(250, 3),
        RT::on_next(260, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_completed(390),
    ]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(255, "bar"), RT::on_next(330, "foo"), RT::on_next(350, "qux"), RT::on_completed(400)]);

    let (a, b, error) = (xs.as_observable(), ys.as_observable(), ex.clone());
    let res = scheduler.start(move || {
        a.try_with_latest_from(b, move |x, y| {
            if x == 5 {
                return Err(error.clone());
            }

            Ok(format!("{x}{y}"))
        })
    });

    assert_messages!(res.messages(), RT::on_next(260, s("4bar")), RT::on_error(310, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn with_latest_from_immediate() {
    let scheduler = TestScheduler::new();

    let xs = Observable::return_(1);
    let ys = Observable::return_("bar");

    let res = scheduler.start(move || xs.with_latest_from(ys, |x, y| format!("{x}{y}")));

    assert_messages!(res.messages(), RT::on_next(200, s("1bar")), RT::on_completed(200));
}

// Not ported: WithLatestFrom_ArgumentChecking - argument validation
