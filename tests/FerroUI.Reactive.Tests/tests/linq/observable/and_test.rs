//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AndTest.cs` of Rx.NET.
//! Adapted: `Observable.When(plan)` is `Observable::when(vec![plan])`; the patterns of upstream
//! (`Pattern<T1, ..., TN>`) are `Pattern2` to `Pattern16`.

use ferroui_reactive::linq::Plan;
use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, new_error, ReactiveTest as RT, TestScheduler};
use std::rc::Rc;

type Source = Rc<dyn IObservable<i32>>;

/// `AndN`: `n` sequences of one value; the plan of all of them sums their values.
fn and_n(n: usize, plan: impl Fn(&[Source]) -> Plan<i32>) {
    let scheduler = TestScheduler::new();

    let obs: Vec<Source> = (0..n).map(|_| scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_completed(220)]).as_observable()).collect();

    let plan = plan(&obs);
    let res = scheduler.start(move || Observable::when(vec![plan]));

    assert_messages!(res.messages(), RT::on_next(210, n as i32), RT::on_completed(220));
}

/// `AndNError`: for each of the `n` sequences, the plan where that one fails ends with the error.
fn and_n_error(n: usize, plan: impl Fn(&[Source]) -> Plan<i32>) {
    let ex = new_error("ex");

    for i in 0..n {
        let scheduler = TestScheduler::new();
        let obs: Vec<Source> = (0..n)
            .map(|j| {
                if j == i {
                    scheduler.create_hot_observable(vec![RT::on_error::<i32>(210, ex.clone())]).as_observable()
                } else {
                    scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_completed(220)]).as_observable()
                }
            })
            .collect();

        let plan = plan(&obs);
        let res = scheduler.start(move || Observable::when(vec![plan]));

        assert_messages!(res.messages(), RT::on_error::<i32>(210, ex.clone()));
    }
}

#[test]
fn and2() {
    and_n(2, |obs| obs[0].and(obs[1].clone()).then(|a, b| a + b));
}

#[test]
fn and2_error() {
    and_n_error(2, |obs| obs[0].and(obs[1].clone()).then(|_a, _b| 0));
}

#[test]
fn and3() {
    and_n(3, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).then(|a, b, c| a + b + c));
}

#[test]
fn and3_error() {
    and_n_error(3, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).then(|_a, _b, _c| 0));
}

#[test]
fn and4() {
    and_n(4, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).then(|a, b, c, d| a + b + c + d));
}

#[test]
fn and4_error() {
    and_n_error(4, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).then(|_a, _b, _c, _d| 0));
}

#[test]
fn and5() {
    and_n(5, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).then(|a, b, c, d, e| a + b + c + d + e));
}

#[test]
fn and5_error() {
    and_n_error(5, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).then(|_a, _b, _c, _d, _e| 0));
}

#[test]
fn and6() {
    and_n(6, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).then(|a, b, c, d, e, f| a + b + c + d + e + f));
}

#[test]
fn and6_error() {
    and_n_error(6, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).then(|_a, _b, _c, _d, _e, _f| 0));
}

#[test]
fn and7() {
    and_n(7, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).then(|a, b, c, d, e, f, g| a + b + c + d + e + f + g));
}

#[test]
fn and7_error() {
    and_n_error(7, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).then(|_a, _b, _c, _d, _e, _f, _g| 0));
}

#[test]
fn and8() {
    and_n(8, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).then(|a, b, c, d, e, f, g, h| a + b + c + d + e + f + g + h));
}

#[test]
fn and8_error() {
    and_n_error(8, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).then(|_a, _b, _c, _d, _e, _f, _g, _h| 0));
}

#[test]
fn and9() {
    and_n(9, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).then(|a, b, c, d, e, f, g, h, i| a + b + c + d + e + f + g + h + i));
}

#[test]
fn and9_error() {
    and_n_error(9, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).then(|_a, _b, _c, _d, _e, _f, _g, _h, _i| 0));
}

#[test]
fn and10() {
    and_n(10, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).then(|a, b, c, d, e, f, g, h, i, j| a + b + c + d + e + f + g + h + i + j));
}

#[test]
fn and10_error() {
    and_n_error(10, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).then(|_a, _b, _c, _d, _e, _f, _g, _h, _i, _j| 0));
}

#[test]
fn and11() {
    and_n(11, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).then(|a, b, c, d, e, f, g, h, i, j, k| a + b + c + d + e + f + g + h + i + j + k));
}

#[test]
fn and11_error() {
    and_n_error(11, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).then(|_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k| 0));
}

#[test]
fn and12() {
    and_n(12, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).then(|a, b, c, d, e, f, g, h, i, j, k, l| a + b + c + d + e + f + g + h + i + j + k + l));
}

#[test]
fn and12_error() {
    and_n_error(12, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).then(|_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l| 0));
}

#[test]
fn and13() {
    and_n(13, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).then(|a, b, c, d, e, f, g, h, i, j, k, l, m| a + b + c + d + e + f + g + h + i + j + k + l + m));
}

#[test]
fn and13_error() {
    and_n_error(13, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).then(|_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l, _m| 0));
}

#[test]
fn and14() {
    and_n(14, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).and(obs[13].clone()).then(|a, b, c, d, e, f, g, h, i, j, k, l, m, n| a + b + c + d + e + f + g + h + i + j + k + l + m + n));
}

#[test]
fn and14_error() {
    and_n_error(14, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).and(obs[13].clone()).then(|_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l, _m, _n| 0));
}

#[test]
fn and15() {
    and_n(15, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).and(obs[13].clone()).and(obs[14].clone()).then(|a, b, c, d, e, f, g, h, i, j, k, l, m, n, o| a + b + c + d + e + f + g + h + i + j + k + l + m + n + o));
}

#[test]
fn and15_error() {
    and_n_error(15, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).and(obs[13].clone()).and(obs[14].clone()).then(|_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l, _m, _n, _o| 0));
}

#[test]
fn and16() {
    and_n(16, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).and(obs[13].clone()).and(obs[14].clone()).and(obs[15].clone()).then(|a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p| a + b + c + d + e + f + g + h + i + j + k + l + m + n + o + p));
}

#[test]
fn and16_error() {
    and_n_error(16, |obs| obs[0].and(obs[1].clone()).and(obs[2].clone()).and(obs[3].clone()).and(obs[4].clone()).and(obs[5].clone()).and(obs[6].clone()).and(obs[7].clone()).and(obs[8].clone()).and(obs[9].clone()).and(obs[10].clone()).and(obs[11].clone()).and(obs[12].clone()).and(obs[13].clone()).and(obs[14].clone()).and(obs[15].clone()).then(|_a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l, _m, _n, _o, _p| 0));
}

// Not ported: And_ArgumentChecking - argument validation
