//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ThrowTest.cs` of Rx.NET.
//! Adapted: an observer that throws is one that panics, and the scheduler that runs it panics,
//! checked with `assert_throws`; `Throw_Witness_Basic2` waits for an event set by the error
//! upstream, which the sequence on the immediate scheduler notifies before `Subscribe` returns: the
//! port checks the error after the subscription.

use ferroui_reactive::{Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_throws, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn throw_basic() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || Observable::throw_on::<i32>(error, s));

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn throw_disposed() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::throw_on::<i32>(new_error("ex"), s), 200);

    assert_messages!(res.messages());
}

#[test]
fn throw_observer_throws() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::throw_on::<i32>(new_error("ex"), scheduler1.scheduler());

    xs.subscribe_all(|_| {}, |_| panic!("InvalidOperationException"), || {});

    assert_throws(|| scheduler1.run());
}

#[test]
fn throw_witness_basic1() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || Observable::throw_on_with_witness(error, s, 42));

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn throw_witness_basic2() {
    let ex = new_error("ex");

    let res: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    let r = res.clone();
    Observable::throw_with_witness(ex.clone(), 42).subscribe_all(
        |_| panic!("unexpected value"),
        move |err| *r.borrow_mut() = Some(err),
        || panic!("unexpected completion"),
    );

    let err = res.borrow().clone().expect("the error was notified");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&ex), Rc::as_ptr(&err)));
}

// Not ported: Throw_ArgumentChecking - argument validation
// Not ported: Throw_DefaultScheduler - real threads (compares with the sequence on DefaultScheduler.Instance, through the blocking ToEnumerable of AssertEqual)
