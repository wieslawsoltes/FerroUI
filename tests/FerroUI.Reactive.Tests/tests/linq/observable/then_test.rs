//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ThenTest.cs` of Rx.NET.
//! Adapted: `Observable.When(plan)` is `Observable::when(vec![plan])`; a selector that throws is
//! the `try_then` form returning the error.

use ferroui_reactive::linq::Plan;
use ferroui_reactive::{IObservable, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, new_error, ReactiveTest as RT, TestScheduler};
use std::rc::Rc;

type Source = Rc<dyn IObservable<i32>>;

#[test]
fn then1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_completed(220)]);

    let x = xs.as_observable();
    let res = scheduler.start(move || Observable::when(vec![x.then(|a| a)]));

    assert_messages!(res.messages(), RT::on_next(210, 1), RT::on_completed(220));
}

#[test]
fn then1_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_error::<i32>(210, ex.clone())]);

    let x = xs.as_observable();
    let res = scheduler.start(move || Observable::when(vec![x.then(|a| a)]));

    assert_messages!(res.messages(), RT::on_error(210, ex));
}

#[test]
fn then1_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_completed(220)]);

    let (x, e) = (xs.as_observable(), ex.clone());
    let res = scheduler.start(move || Observable::when(vec![x.try_then(move |_a| Err::<i32, _>(e.clone()))]));

    assert_messages!(res.messages(), RT::on_error::<i32>(210, ex));
}

/// `ThenNThrows`: `n` sequences of one value, and a plan of all of them whose selector fails.
fn then_n_throws(n: usize, plan: impl Fn(&[Source], ObservableError) -> Plan<i32>) {
    let scheduler = TestScheduler::new();
    let ex = new_error("ex");

    let obs: Vec<Source> = (0..n).map(|_| scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_completed(220)]).as_observable()).collect();

    let plan = plan(&obs, ex.clone());
    let res = scheduler.start(move || Observable::when(vec![plan]));

    assert_messages!(res.messages(), RT::on_error::<i32>(210, ex));
}

#[test]
fn then2_throws() {
    then_n_throws(2, |obs, ex| obs[0].and(obs[1].clone()).try_then(move |_a, _b| Err::<i32, _>(ex.clone())));
}

#[test]
fn then3_throws() {
    then_n_throws(3, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).try_then(move |_a, _b, _c| Err::<i32, _>(ex.clone())));
}

#[test]
fn then4_throws() {
    then_n_throws(4, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).try_then(move |_a, _b, _c, _d| Err::<i32, _>(ex.clone())));
}

#[test]
fn then5_throws() {
    then_n_throws(5, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).try_then(move |_a, _b, _c, _d, _e| Err::<i32, _>(ex.clone())));
}

#[test]
fn then6_throws() {
    then_n_throws(6, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).try_then(move |_a, _b, _c, _d, _e, _f| Err::<i32, _>(ex.clone())));
}

#[test]
fn then7_throws() {
    then_n_throws(7, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g| Err::<i32, _>(ex.clone())));
}

#[test]
fn then8_throws() {
    then_n_throws(8, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g, _h| Err::<i32, _>(ex.clone())));
}

#[test]
fn then9_throws() {
    then_n_throws(9, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g, _h, _i| Err::<i32, _>(ex.clone())));
}

#[test]
fn then10_throws() {
    then_n_throws(10, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g, _h, _i, _j| Err::<i32, _>(ex.clone())));
}

#[test]
fn then11_throws() {
    then_n_throws(11, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k| Err::<i32, _>(ex.clone())));
}

#[test]
fn then12_throws() {
    then_n_throws(12, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l| Err::<i32, _>(ex.clone())));
}

#[test]
fn then13_throws() {
    then_n_throws(13, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l, _m| Err::<i32, _>(ex.clone())));
}

#[test]
fn then14_throws() {
    then_n_throws(14, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).and(obs[13].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l, _m, _n| Err::<i32, _>(ex.clone())));
}

#[test]
fn then15_throws() {
    then_n_throws(15, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).and(obs[13].clone()).and(obs[14].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l, _m, _n, _o| Err::<i32, _>(ex.clone())));
}

#[test]
fn then16_throws() {
    then_n_throws(16, |obs, ex| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).and(obs[13].clone()).and(obs[14].clone()).and(obs[15].clone()).try_then(move |_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l, _m, _n, _o, _p| Err::<i32, _>(ex.clone())));
}

// Not ported: Then_ArgumentChecking - argument validation
