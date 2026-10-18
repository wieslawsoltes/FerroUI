//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ManySelectTest.cs` of Rx.NET.
//! Adapted: `ManySelect(selector, scheduler)` is `many_select_on`. The blocking `First` of upstream
//! is `first` below, a subscription to `first_async` that reads the value it delivers before it
//! returns: the sequences it reads start with a value notified at once (a chain of `ManySelect`
//! delivers its head through the trampoline of the current thread, which the subscription runs),
//! or end at once.

use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;

/// `First()`: the first value of a sequence that notifies it while it is subscribed.
fn first<T: Clone + 'static>(ys: Rc<dyn IObservable<T>>) -> T {
    let value = Rc::new(RefCell::new(None));
    let target = value.clone();
    let _d = ys.first_async().subscribe_next(move |x| *target.borrow_mut() = Some(x));
    let first = value.borrow_mut().take();
    first.expect("the sequence notifies its first value while it is subscribed")
}

#[test]
fn many_select_law_1() {
    let xs = Observable::range(1, 0);

    let left = xs.many_select(first::<i32>);
    let right = xs;

    assert!(first(left.sequence_equal(right)));
}

#[test]
fn many_select_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(100, 1), RT::on_next(220, 2), RT::on_next(270, 3), RT::on_next(410, 4), RT::on_completed(500)]);

    let (x, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || x.many_select_on(first::<i32>, s));

    assert_messages!(res.messages(), RT::on_next(221, 2), RT::on_next(271, 3), RT::on_next(411, 4), RT::on_completed(501));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn many_select_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(100, 1), RT::on_next(220, 2), RT::on_next(270, 3), RT::on_next(410, 4), RT::on_error(500, ex.clone())]);

    let (x, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || x.many_select_on(first::<i32>, s));

    assert_messages!(res.messages(), RT::on_next(221, 2), RT::on_next(271, 3), RT::on_next(411, 4), RT::on_error(501, ex));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 500));
}

// Not ported: ManySelect_ArgumentChecking - argument validation
// Not ported: ManySelect_Law_2 - real threads (the chains are delivered on DefaultScheduler.Instance, and the selector waits on them with the blocking First; on the one thread of the port the head of a chain waits in the trampoline of the subscription the selector runs in)
// Not ported: ManySelect_Law_3 - real threads (the chains are delivered on DefaultScheduler.Instance, and the selectors wait on them with the blocking First and Last)
