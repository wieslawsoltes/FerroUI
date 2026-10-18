//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ExpandTest.cs` of Rx.NET.
//! Adapted: `Expand(selector, scheduler)` is `expand_on`, and a selector that throws is the
//! `try_expand_on` form returning the error; `Expand_Default` reads the result of `SequenceEqual`
//! with a subscription, which delivers it before it returns (the sequences run on the current
//! thread; upstream's blocking `First` waits for it).

use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn expand_default() {
    let b = Rc::new(Cell::new(None));
    let target = b.clone();
    let _d = Observable::return_(1)
        .expand(|x| if x < 10 { Observable::return_(x + 1) } else { Observable::empty::<i32>() })
        .sequence_equal(Observable::range(1, 10))
        .first_async()
        .subscribe_next(move |equal| target.set(Some(equal)));

    assert_eq!(Some(true), b.get());
}

#[test]
fn expand_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(300)]);

    let (x, s, inner) = (xs.as_observable(), scheduler.scheduler(), scheduler.clone());
    let res = scheduler.start(move || {
        x.expand_on(
            move |_x| -> Rc<dyn IObservable<i32>> {
                inner.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(200, 2), RT::on_completed(300)]).as_observable()
            },
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_completed(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(201, 300));
}

#[test]
fn expand_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_error::<i32>(300, ex.clone())]);

    let (x, s, inner) = (xs.as_observable(), scheduler.scheduler(), scheduler.clone());
    let res = scheduler.start(move || {
        x.expand_on(
            move |x| -> Rc<dyn IObservable<i32>> {
                let t = i64::from(x);
                inner
                    .create_cold_observable(vec![RT::on_next(100 + t, 2 * x), RT::on_next(200 + t, 3 * x), RT::on_completed(300 + t)])
                    .as_observable()
            },
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_error(300, ex));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(201, 300));
}

#[test]
fn expand_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let (x, s, inner) = (xs.as_observable(), scheduler.scheduler(), scheduler.clone());
    let res = scheduler.start(move || {
        x.expand_on(
            move |x| -> Rc<dyn IObservable<i32>> {
                let t = i64::from(x);
                inner
                    .create_cold_observable(vec![RT::on_next(100 + t, 2 * x), RT::on_next(200 + t, 3 * x), RT::on_completed(300 + t)])
                    .as_observable()
            },
            s,
        )
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(201, 1000));
}

#[test]
fn expand_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(550, 1), RT::on_next(850, 2), RT::on_completed(950)]);

    let (x, s, inner) = (xs.as_observable(), scheduler.scheduler(), scheduler.clone());
    let res = scheduler.start(move || {
        x.expand_on(
            move |x| -> Rc<dyn IObservable<i32>> {
                inner.create_cold_observable(vec![RT::on_next(100, 2 * x), RT::on_next(200, 3 * x), RT::on_completed(300)]).as_observable()
            },
            s,
        )
    });

    assert_messages!(
        res.messages(),
        RT::on_next(550, 1),
        RT::on_next(651, 2),
        RT::on_next(751, 3),
        RT::on_next(752, 4),
        RT::on_next(850, 2),
        RT::on_next(852, 6),
        RT::on_next(852, 6),
        RT::on_next(853, 8),
        RT::on_next(951, 4),
        RT::on_next(952, 9),
        RT::on_next(952, 12),
        RT::on_next(953, 12),
        RT::on_next(953, 12),
        RT::on_next(954, 16)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(201, 950));
}

#[test]
fn expand_throw() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(550, 1), RT::on_next(850, 2), RT::on_completed(950)]);

    let ex = new_error("ex");

    let (x, s, e) = (xs.as_observable(), scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || x.try_expand_on(move |_x| Err(e.clone()), s));

    assert_messages!(res.messages(), RT::on_next(550, 1), RT::on_error(550, ex));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(201, 550));
}

// Not ported: Expand_ArgumentChecking - argument validation
