//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ForTest.cs` of Rx.NET.
//! Adapted: `Observable.For` is `Observable::for_`; an iterator that throws after its items
//! (`For_Error_Core`) and a selector that throws are the `try_for` form, whose items and selector
//! return the error.

use ferroui_reactive::{IObservable, Observable, ObservableError};
use ferroui_reactive_testing::{assert_messages, new_error, ReactiveTest as RT, TestScheduler};
use std::rc::Rc;

/// The cold sequence of the item `x`.
fn cold_of(scheduler: &TestScheduler, x: i32) -> Rc<dyn IObservable<i32>> {
    let x64 = i64::from(x);
    scheduler
        .create_cold_observable(vec![
            RT::on_next(x64 * 100 + 10, x * 10 + 1),
            RT::on_next(x64 * 100 + 20, x * 10 + 2),
            RT::on_next(x64 * 100 + 30, x * 10 + 3),
            RT::on_completed(x64 * 100 + 40),
        ])
        .as_observable()
}

#[test]
fn for_basic() {
    let scheduler = TestScheduler::new();

    let s = scheduler.clone();
    let results = scheduler.start(move || Observable::for_(vec![1, 2, 3], move |x| cold_of(&s, x)));

    assert_messages!(
        results.messages(),
        RT::on_next(310, 11),
        RT::on_next(320, 12),
        RT::on_next(330, 13),
        RT::on_next(550, 21),
        RT::on_next(560, 22),
        RT::on_next(570, 23),
        RT::on_next(890, 31),
        RT::on_next(900, 32),
        RT::on_next(910, 33),
        RT::on_completed(920)
    );
}

/// `For_Error_Core`: the items 1, 2 and 3, then the error.
fn for_error_core(ex: &ObservableError) -> Vec<Result<i32, ObservableError>> {
    vec![Ok(1), Ok(2), Ok(3), Err(ex.clone())]
}

#[test]
fn for_error_iterator() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, items) = (scheduler.clone(), for_error_core(&ex));
    let results = scheduler.start(move || Observable::try_for(items, move |x| Ok(cold_of(&s, x))));

    assert_messages!(
        results.messages(),
        RT::on_next(310, 11),
        RT::on_next(320, 12),
        RT::on_next(330, 13),
        RT::on_next(550, 21),
        RT::on_next(560, 22),
        RT::on_next(570, 23),
        RT::on_next(890, 31),
        RT::on_next(900, 32),
        RT::on_next(910, 33),
        RT::on_error(920, ex)
    );
}

#[test]
fn for_error_source() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e = ex.clone();
    let results = scheduler.start(move || Observable::for_(vec![1, 2, 3], move |_x: i32| Observable::throw::<i32>(e.clone())));

    assert_messages!(results.messages(), RT::on_error::<i32>(200, ex));
}

#[test]
fn for_selector_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e = ex.clone();
    let items: Vec<Result<i32, ObservableError>> = vec![Ok(1), Ok(2), Ok(3)];
    let results = scheduler.start(move || Observable::try_for(items, move |_x: i32| Err::<Rc<dyn IObservable<i32>>, _>(e.clone())));

    assert_messages!(results.messages(), RT::on_error::<i32>(200, ex));
}

// Not ported: For_ArgumentChecking - argument validation
