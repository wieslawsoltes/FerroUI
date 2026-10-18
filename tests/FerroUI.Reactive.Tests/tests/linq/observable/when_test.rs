//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/WhenTest.cs` of Rx.NET.
//! Adapted: `Observable.When(params plans)` is `Observable::when(vec![...])`; the enumerable of
//! plans that throws (`GetPlans`) is `Observable::try_when` with an enumeration that fails;
//! `SameSource` reads the list with a subscription, which delivers it before it returns (`Range`
//! runs on the current thread; upstream's blocking `First` waits for it).

use ferroui_reactive::linq::Plan;
use ferroui_reactive::{IObservable, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn when_multiple_data_symmetric() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_next(250, 5), RT::on_next(260, 6), RT::on_completed(270)]);

    let (x, y) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || Observable::when(vec![x.and(y).then(|x, y| x + y)]));

    assert_messages!(res.messages(), RT::on_next(240, 1 + 4), RT::on_next(250, 2 + 5), RT::on_next(260, 3 + 6), RT::on_completed(270));
}

#[test]
fn when_multiple_data_asymmetric() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_next(250, 5), RT::on_completed(270)]);

    let (x, y) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || Observable::when(vec![x.and(y).then(|x, y| x + y)]));

    assert_messages!(res.messages(), RT::on_next(240, 1 + 4), RT::on_next(250, 2 + 5), RT::on_completed(270));
}

#[test]
fn when_empty_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(240)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(270)]);

    let (x, y) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || Observable::when(vec![x.and(y).then(|x, y| x + y)]));

    assert_messages!(res.messages(), RT::on_completed(270));
}

#[test]
fn when_never_never() {
    let scheduler = TestScheduler::new();

    let xs = Observable::never::<i32>();
    let ys = Observable::never::<i32>().as_observable();

    let res = scheduler.start(move || Observable::when(vec![xs.and(ys).then(|x, y| x + y)]));

    assert_messages!(res.messages());
}

#[test]
fn when_throw_non_empty() {
    let ex = new_error("ex");
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_error::<i32>(240, ex.clone())]);

    let ys = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(270)]);

    let (x, y) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || Observable::when(vec![x.and(y).then(|x, y| x + y)]));

    assert_messages!(res.messages(), RT::on_error(240, ex));
}

#[test]
fn complicated_when() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_next(250, 5), RT::on_next(260, 6), RT::on_completed(270)]);

    let zs = scheduler.create_hot_observable(vec![RT::on_next(220, 7), RT::on_next(230, 8), RT::on_next(240, 9), RT::on_completed(300)]);

    let (x, y, z) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || {
        Observable::when(vec![
            x.and(y.clone()).then(|x, y| x + y),
            x.and(z.clone()).then(|x, z| x * z),
            y.and(z).then(|y, z| y - z),
        ])
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, 1 * 7),
        RT::on_next(230, 2 * 8),
        RT::on_next(240, 3 + 4),
        RT::on_next(250, 5 - 9),
        RT::on_completed(300)
    );
}

#[test]
fn when_plans_iterator_throws() {
    let ex = new_error("ex");
    let error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    // `GetPlans`: an iterator of plans that throws before its first plan.
    let e = ex.clone();
    let plans = Observable::try_when(move || Err::<Vec<Result<Plan<i32>, ObservableError>>, _>(e.clone()));

    let target = error.clone();
    let _d = plans.subscribe_next_error(|_| {}, move |e| *target.borrow_mut() = Some(e));

    let error = error.borrow().clone().expect("the error is notified");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&error), Rc::as_ptr(&ex)));
}

#[test]
fn same_source() {
    let source: Rc<dyn IObservable<i32>> = Observable::range(1, 5);

    let list = Rc::new(RefCell::new(None));
    let target = list.clone();
    let _d = Observable::when(vec![source.and(source.clone()).then(|a, b| a + b)])
        .to_list()
        .subscribe_next(move |values| *target.borrow_mut() = Some(values));

    assert_eq!(Some(vec![2, 4, 6, 8, 10]), list.borrow().clone());
}

// Not ported: When_ArgumentChecking - argument validation
