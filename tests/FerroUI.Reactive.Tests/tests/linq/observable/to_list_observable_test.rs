//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ToListObservableTest.cs` of Rx.NET.
//! Adapted: the sequence of a `ListObservable<T>` (an `IObservable<object>` upstream, which notifies
//! only the end of the source) is a sequence of `()`; the members of the list return the error of
//! the source as an `Err` where upstream rethrows it; `s.AssertEqual(1, 2, 3)` compares the values
//! of the list (`to_vec`).

use ferroui_reactive::linq::ListObservable;
use ferroui_reactive::{IDisposable, IObservable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// `InvalidOperationException`.
#[derive(Debug)]
struct InvalidOperationException;

impl fmt::Display for InvalidOperationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Operation is not valid due to the current state of the object.")
    }
}

impl std::error::Error for InvalidOperationException {}

#[test]
fn to_list_observable_on_next() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(300, 1), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_next(600, 4)]);

    let x = xs.as_observable();
    let res = scheduler.start(move || -> Rc<dyn IObservable<()>> { x.to_list_observable() });

    assert_messages!(res.messages());
}

#[test]
fn to_list_observable_on_error() {
    let scheduler = TestScheduler::new();

    let ex: ObservableError = Rc::new(InvalidOperationException);

    let xs = scheduler.create_hot_observable(vec![RT::on_next(300, 1), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_error(600, ex.clone())]);

    let s: Rc<RefCell<Option<Rc<ListObservable<i32>>>>> = Rc::new(RefCell::new(None));
    let subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    let res = scheduler.create_observer::<()>();

    let (s1, x) = (s.clone(), xs.as_observable());
    scheduler.schedule_absolute_action(RT::CREATED, move || *s1.borrow_mut() = Some(x.to_list_observable()));
    let (s2, d1, observer) = (s.clone(), subscription.clone(), res.clone());
    scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || {
        let list = s2.borrow().clone().expect("the list is made");
        *d1.borrow_mut() = Some(list.subscribe(observer));
    });
    let d2 = subscription.clone();
    scheduler.schedule_absolute_action(RT::DISPOSED, move || {
        let subscription = d2.borrow().clone().expect("the list is subscribed");
        subscription.dispose();
    });

    scheduler.run();

    let list = s.borrow().clone().expect("the list is made");
    let error = list.value().expect_err("the value is the error of the source");
    assert!(error.downcast_ref::<InvalidOperationException>().is_some());

    assert_messages!(res.messages(), RT::on_error(600, ex));
}

#[test]
fn to_list_observable_on_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(300, 1), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_completed(600)]);

    let s: Rc<RefCell<Option<Rc<ListObservable<i32>>>>> = Rc::new(RefCell::new(None));

    let (s1, x) = (s.clone(), xs.as_observable());
    let res = scheduler.start(move || -> Rc<dyn IObservable<()>> {
        let list = x.to_list_observable();
        *s1.borrow_mut() = Some(list.clone());
        list
    });

    let list = s.borrow().clone().expect("the list is made");
    assert_eq!(vec![1, 2, 3], list.to_vec().expect("the source has completed"));

    assert_messages!(res.messages(), RT::on_completed(600));

    assert_eq!(3, list.value().expect("the list has a value"));
}

#[test]
fn to_list_observable_disposed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(300, 1),
        RT::on_next(400, 2),
        RT::on_next(500, 3),
        RT::on_next(1050, 4),
        RT::on_completed(1100),
    ]);

    let s: Rc<RefCell<Option<Rc<ListObservable<i32>>>>> = Rc::new(RefCell::new(None));

    let (s1, x) = (s.clone(), xs.as_observable());
    let res = scheduler.start(move || -> Rc<dyn IObservable<()>> {
        let list = x.to_list_observable();
        *s1.borrow_mut() = Some(list.clone());
        list
    });

    assert_messages!(res.messages());
}

#[test]
fn to_list_observable_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let x = xs.as_observable();
    let res = scheduler.start(move || -> Rc<dyn IObservable<()>> { x.to_list_observable() });

    assert_messages!(res.messages());
}

// Not ported: ToListObservable_ArgumentChecking - argument validation
