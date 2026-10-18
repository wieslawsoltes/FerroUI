//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/FinallyTest.cs` of Rx.NET.
//! Adapted: the flags, counters and strings the actions update are shared `Cell`s and `RefCell`s.

use ferroui_reactive::{Observable, ObservableExt, Unit};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn flag() -> Rc<Cell<bool>> {
    Rc::new(Cell::new(false))
}

#[test]
fn finally_never() {
    let scheduler = TestScheduler::new();

    let invoked = flag();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || Observable::never::<i32>().finally(move || invoked.set(true)))
    };

    assert_messages!(res.messages());

    assert!(invoked.get()); // due to unsubscribe; see 1356
}

#[test]
fn finally_only_called_once_never() {
    let invoke_count = Rc::new(Cell::new(0));
    let some_observable = {
        let invoke_count = invoke_count.clone();
        Observable::never::<i32>().finally(move || invoke_count.set(invoke_count.get() + 1))
    };
    let d = some_observable.subscribe_ignore();
    d.dispose();
    d.dispose();

    assert_eq!(1, invoke_count.get());
}

#[test]
fn finally_only_called_once_empty() {
    let invoke_count = Rc::new(Cell::new(0));
    let some_observable = {
        let invoke_count = invoke_count.clone();
        Observable::empty::<i32>().finally(move || invoke_count.set(invoke_count.get() + 1))
    };
    let d = some_observable.subscribe_ignore();
    d.dispose();
    d.dispose();

    assert_eq!(1, invoke_count.get());
}

#[test]
fn finally_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let invoked = flag();
    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || source.as_observable().finally(move || invoked.set(true)))
    };

    assert!(invoked.get());

    assert_messages!(res.messages(), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn finally_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let invoked = flag();
    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || source.as_observable().finally(move || invoked.set(true)))
    };

    assert!(invoked.get());

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn finally_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");
    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, ex.clone())]);

    let invoked = flag();
    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || source.as_observable().finally(move || invoked.set(true)))
    };

    assert!(invoked.get());

    assert_messages!(res.messages(), RT::on_error(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

/// `.Finally(() => order += "1").Finally(() => order += "2").Finally(() => order += "3")`.
fn with_order(source: Rc<dyn ferroui_reactive::IObservable<Unit>>, order: &Rc<RefCell<String>>) -> Rc<dyn ferroui_reactive::IObservable<Unit>> {
    let (o1, o2, o3) = (order.clone(), order.clone(), order.clone());
    source
        .finally(move || o1.borrow_mut().push('1'))
        .finally(move || o2.borrow_mut().push('2'))
        .finally(move || o3.borrow_mut().push('3'))
}

#[test]
fn finally_dispose_order_empty() {
    let order = Rc::new(RefCell::new(String::new()));
    with_order(Observable::empty::<Unit>(), &order).subscribe_ignore();

    assert_eq!("123", *order.borrow());
}

#[test]
fn finally_dispose_order_return() {
    let order = Rc::new(RefCell::new(String::new()));
    with_order(Observable::return_(()), &order).subscribe_ignore();

    assert_eq!("123", *order.borrow());
}

#[test]
fn finally_dispose_order_never() {
    let order = Rc::new(RefCell::new(String::new()));
    let d = with_order(Observable::never::<Unit>(), &order).subscribe_ignore();

    d.dispose();

    assert_eq!("123", *order.borrow());
}

// Not ported: Finally_ArgumentChecking - argument validation
