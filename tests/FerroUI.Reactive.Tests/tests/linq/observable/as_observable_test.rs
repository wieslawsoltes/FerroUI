//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AsObservableTest.cs` of Rx.NET.
//! Adapted: `Assert.NotSame` compares the addresses of the handles; the flag that the subscription
//! sets is a shared `Cell`. The test observables are coerced to the handle `Rc<dyn IObservable<T>>`
//! first (their own `as_observable` gives that handle, not the operator).

use ferroui_reactive::{IDisposable, IObservable, IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn as_observable_as_observable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let source: Rc<dyn IObservable<i32>> = xs.clone();
    let ys = source.as_observable().as_observable();

    assert!(!std::ptr::addr_eq(Rc::as_ptr(&xs), Rc::as_ptr(&ys)));

    let res = scheduler.start(move || ys);

    assert_messages!(res.messages(), RT::on_next(220, 2), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn as_observable_hides() {
    let xs = Observable::empty::<i32>();

    let res = xs.as_observable();

    assert!(!std::ptr::addr_eq(Rc::as_ptr(&xs), Rc::as_ptr(&res)));
}

#[test]
fn as_observable_never() {
    let scheduler = TestScheduler::new();

    let xs = Observable::never::<i32>();

    let res = scheduler.start(move || xs.as_observable());

    assert_messages!(res.messages());
}

#[test]
fn as_observable_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let source: Rc<dyn IObservable<i32>> = xs.clone();
    let res = scheduler.start(move || source.as_observable());

    assert_messages!(res.messages(), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn as_observable_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");
    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, ex.clone())]);

    let source: Rc<dyn IObservable<i32>> = xs.clone();
    let res = scheduler.start(move || source.as_observable());

    assert_messages!(res.messages(), RT::on_error(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn as_observable_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let source: Rc<dyn IObservable<i32>> = xs.clone();
    let res = scheduler.start(move || source.as_observable());

    assert_messages!(res.messages(), RT::on_next(220, 2), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn as_observable_is_not_eager() {
    let scheduler = TestScheduler::new();

    let subscribed = Rc::new(Cell::new(false));
    let xs: Rc<dyn IObservable<i32>> = {
        let (subscribed, scheduler) = (subscribed.clone(), scheduler.clone());
        Observable::create_with_action(move |obs: Rc<dyn IObserver<i32>>| {
            subscribed.set(true);

            let disp = scheduler
                .create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)])
                .subscribe(obs);

            move || disp.dispose()
        })
    };

    let _ = xs.as_observable();
    assert!(!subscribed.get());

    let source = xs.clone();
    let _res = scheduler.start(move || source.as_observable());
    assert!(subscribed.get());
}

// Not ported: AsObservable_ArgumentChecking - argument validation
