//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DeferTest.cs` of Rx.NET.
//! Adapted: the variables that the factory assigns are shared slots; a factory that throws is the
//! factory of `Observable::try_defer` returning the error.

use ferroui_reactive::{IObservable, Observable};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ColdObservable, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Slot<T> = Rc<RefCell<Option<T>>>;

fn slot<T>() -> Slot<T> {
    Rc::new(RefCell::new(None))
}

fn get<T: Clone>(slot: &Slot<T>) -> T {
    slot.borrow().clone().expect("the variable has been assigned")
}

#[test]
fn defer_complete() {
    let scheduler = TestScheduler::new();

    let invoked = Rc::new(Cell::new(0));
    let xs: Slot<Rc<ColdObservable<i64>>> = slot();

    let (s, i, x) = (scheduler.clone(), invoked.clone(), xs.clone());
    let res = scheduler.start(move || {
        Observable::defer(move || -> Rc<dyn IObservable<i64>> {
            i.set(i.get() + 1);
            let cold = s.create_cold_observable(vec![RT::on_next(100, s.clock()), RT::on_completed(200)]);
            *x.borrow_mut() = Some(cold.clone());
            cold
        })
    });

    assert_messages!(res.messages(), RT::on_next(300, 200i64), RT::on_completed::<i64>(400));

    assert_eq!(1, invoked.get());

    assert_subscriptions!(get(&xs).subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn defer_error() {
    let scheduler = TestScheduler::new();

    let invoked = Rc::new(Cell::new(0));
    let xs: Slot<Rc<ColdObservable<i64>>> = slot();
    let ex = new_error("ex");

    let (s, i, x, error) = (scheduler.clone(), invoked.clone(), xs.clone(), ex.clone());
    let res = scheduler.start(move || {
        Observable::defer(move || -> Rc<dyn IObservable<i64>> {
            i.set(i.get() + 1);
            let cold = s.create_cold_observable(vec![RT::on_next(100, s.clock()), RT::on_error(200, error.clone())]);
            *x.borrow_mut() = Some(cold.clone());
            cold
        })
    });

    assert_messages!(res.messages(), RT::on_next(300, 200i64), RT::on_error::<i64>(400, ex));

    assert_eq!(1, invoked.get());

    assert_subscriptions!(get(&xs).subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn defer_dispose() {
    let scheduler = TestScheduler::new();

    let invoked = Rc::new(Cell::new(0i64));
    let xs: Slot<Rc<ColdObservable<i64>>> = slot();

    let (s, i, x) = (scheduler.clone(), invoked.clone(), xs.clone());
    let res = scheduler.start(move || {
        Observable::defer(move || -> Rc<dyn IObservable<i64>> {
            i.set(i.get() + 1);
            let cold = s.create_cold_observable(vec![RT::on_next(100, s.clock()), RT::on_next(200, i.get()), RT::on_next(1100, 1000)]);
            *x.borrow_mut() = Some(cold.clone());
            cold
        })
    });

    assert_messages!(res.messages(), RT::on_next(300, 200i64), RT::on_next(400, 1i64));

    assert_eq!(1, invoked.get());

    assert_subscriptions!(get(&xs).subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn defer_throw() {
    let scheduler = TestScheduler::new();

    let invoked = Rc::new(Cell::new(0));
    let ex = new_error("ex");

    let (i, error) = (invoked.clone(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_defer(move || -> Result<Rc<dyn IObservable<i32>>, _> {
            i.set(i.get() + 1);
            Err(error.clone())
        })
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));

    assert_eq!(1, invoked.get());
}

// Not ported: Defer_ArgumentChecking - argument validation (a null factory, a null observer, a factory that returns null)
