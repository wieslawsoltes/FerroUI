//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/EmptyTest.cs` of Rx.NET.
//! Adapted: an observer that throws is one that panics, and the scheduler that runs it panics,
//! checked with `assert_throws`; `Empty_Basic_Witness2` waits for an event set by the completion
//! upstream, which the sequence on the immediate scheduler notifies before `Subscribe` returns: the
//! port checks a flag after the subscription.

use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_throws, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn empty_basic() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::empty_on::<i32>(s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(201));
}

#[test]
fn empty_disposed() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::empty_on::<i32>(s), 200);

    assert_messages!(res.messages());
}

#[test]
fn empty_observer_throws() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::empty_on::<i32>(scheduler1.scheduler());

    xs.subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));

    assert_throws(|| scheduler1.run());
}

#[test]
fn empty_basic_witness1() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::empty_on_with_witness(s, 42));

    assert_messages!(res.messages(), RT::on_completed::<i32>(201));
}

#[test]
fn empty_basic_witness2() {
    let completed = Rc::new(Cell::new(false));

    let c = completed.clone();
    Observable::empty_with_witness(42).subscribe_all(|_| panic!("unexpected value"), |_| panic!("unexpected error"), move || c.set(true));

    assert!(completed.get());
}

// Not ported: Empty_ArgumentChecking - argument validation
// Not ported: Empty_DefaultScheduler - real threads (compares with the sequence on DefaultScheduler.Instance, through the blocking ToEnumerable of AssertEqual)
