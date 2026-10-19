//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ToAsyncTest.cs` of Rx.NET.
//! Adapted: the overloads of `ToAsync` are `Observable::to_async` and `to_async_on`, which choose
//! the arity from the closure (its argument types are written out), and the functions that throw
//! go through `try_to_async`; an `Action` is a closure returning `()`. `Scheduler.Default` is the
//! default scheduler of the thread (`SchedulerDefaults::default_scheduler`, the current-thread
//! scheduler in a test) where upstream has the thread pool, so the function runs before the call
//! returns; the blocking `ToEnumerable()` is `block_on` of `to_list().to_task()`. The flag the
//! actions set is a shared `Cell`.

use ferroui_reactive::concurrency::SchedulerDefaults;
use ferroui_reactive::{IObservable, Notification, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::threading::tasks::block_on;
use std::cell::Cell;
use std::rc::Rc;

/// The values of a sequence that has ended (`ToEnumerable()`).
fn to_enumerable<T: Clone + 'static>(source: Rc<dyn IObservable<T>>) -> Vec<T> {
    block_on(source.to_list().to_task()).unwrap()
}

#[test]
fn to_async0() {
    assert_eq!(vec![0], to_enumerable(Observable::to_async(|| 0)()));
    assert_eq!(vec![0], to_enumerable(Observable::to_async_on(|| 0, SchedulerDefaults::default_scheduler())()));
}

#[test]
fn to_async1() {
    assert_eq!(vec![1], to_enumerable(Observable::to_async(|a: i32| a)(1)));
    assert_eq!(vec![1], to_enumerable(Observable::to_async_on(|a: i32| a, SchedulerDefaults::default_scheduler())(1)));
}

#[test]
fn to_async2() {
    assert_eq!(vec![1 + 2], to_enumerable(Observable::to_async(|a: i32, b: i32| a + b)(1, 2)));
    assert_eq!(vec![1 + 2], to_enumerable(Observable::to_async_on(|a: i32, b: i32| a + b, SchedulerDefaults::default_scheduler())(1, 2)));
}

#[test]
fn to_async3() {
    assert_eq!(vec![1 + 2 + 3], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32| a + b + c)(1, 2, 3)));
    assert_eq!(vec![1 + 2 + 3], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32| a + b + c, SchedulerDefaults::default_scheduler())(1, 2, 3)));
}

#[test]
fn to_async4() {
    assert_eq!(vec![1 + 2 + 3 + 4], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32| a + b + c + d)(1, 2, 3, 4)));
    assert_eq!(vec![1 + 2 + 3 + 4], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32| a + b + c + d, SchedulerDefaults::default_scheduler())(1, 2, 3, 4)));
}

#[test]
fn to_async5() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32| a + b + c + d + e)(1, 2, 3, 4, 5)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32| a + b + c + d + e, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5)));
}

#[test]
fn to_async6() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32| a + b + c + d + e + f)(1, 2, 3, 4, 5, 6)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32| a + b + c + d + e + f, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6)));
}

#[test]
fn to_async7() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32| a + b + c + d + e + f + g)(1, 2, 3, 4, 5, 6, 7)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32| a + b + c + d + e + f + g, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7)));
}

#[test]
fn to_async8() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32| a + b + c + d + e + f + g + h)(1, 2, 3, 4, 5, 6, 7, 8)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32| a + b + c + d + e + f + g + h, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7, 8)));
}

#[test]
fn to_async9() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32| a + b + c + d + e + f + g + h + i)(1, 2, 3, 4, 5, 6, 7, 8, 9)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32| a + b + c + d + e + f + g + h + i, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7, 8, 9)));
}

#[test]
fn to_async10() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32| a + b + c + d + e + f + g + h + i + j)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32| a + b + c + d + e + f + g + h + i + j, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)));
}

#[test]
fn to_async11() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32| a + b + c + d + e + f + g + h + i + j + k)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32| a + b + c + d + e + f + g + h + i + j + k, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)));
}

#[test]
fn to_async12() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32| a + b + c + d + e + f + g + h + i + j + k + l)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32| a + b + c + d + e + f + g + h + i + j + k + l, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)));
}

#[test]
fn to_async13() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32| a + b + c + d + e + f + g + h + i + j + k + l + m)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32| a + b + c + d + e + f + g + h + i + j + k + l + m, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)));
}

#[test]
fn to_async14() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32| a + b + c + d + e + f + g + h + i + j + k + l + m + n)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32| a + b + c + d + e + f + g + h + i + j + k + l + m + n, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14)));
}

#[test]
fn to_async15() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32| a + b + c + d + e + f + g + h + i + j + k + l + m + n + o)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32| a + b + c + d + e + f + g + h + i + j + k + l + m + n + o, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15)));
}

#[test]
fn to_async16() {
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16], to_enumerable(Observable::to_async(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32, p: i32| a + b + c + d + e + f + g + h + i + j + k + l + m + n + o + p)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)));
    assert_eq!(vec![1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16], to_enumerable(Observable::to_async_on(|a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32, p: i32| a + b + c + d + e + f + g + h + i + j + k + l + m + n + o + p, SchedulerDefaults::default_scheduler())(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)));
}

#[test]
fn to_async_error0() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move || Err::<i32, ObservableError>(err.clone()))().materialize())
    );
}

#[test]
fn to_async_error1() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32| Err::<i32, ObservableError>(err.clone()))(1).materialize())
    );
}

#[test]
fn to_async_error2() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32| Err::<i32, ObservableError>(err.clone()))(1, 2).materialize())
    );
}

#[test]
fn to_async_error3() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3).materialize())
    );
}

#[test]
fn to_async_error4() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4).materialize())
    );
}

#[test]
fn to_async_error5() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5).materialize())
    );
}

#[test]
fn to_async_error6() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6).materialize())
    );
}

#[test]
fn to_async_error7() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7).materialize())
    );
}

#[test]
fn to_async_error8() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32, _h: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7, 8).materialize())
    );
}

#[test]
fn to_async_error9() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32, _h: i32, _i: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7, 8, 9).materialize())
    );
}

#[test]
fn to_async_error10() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32, _h: i32, _i: i32, _j: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7, 8, 9, 10).materialize())
    );
}

#[test]
fn to_async_error11() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32, _h: i32, _i: i32, _j: i32, _k: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11).materialize())
    );
}

#[test]
fn to_async_error12() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32, _h: i32, _i: i32, _j: i32, _k: i32, _l: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12).materialize())
    );
}

#[test]
fn to_async_error13() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32, _h: i32, _i: i32, _j: i32, _k: i32, _l: i32, _m: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13).materialize())
    );
}

#[test]
fn to_async_error14() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32, _h: i32, _i: i32, _j: i32, _k: i32, _l: i32, _m: i32, _n: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14).materialize())
    );
}

#[test]
fn to_async_error15() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32, _h: i32, _i: i32, _j: i32, _k: i32, _l: i32, _m: i32, _n: i32, _o: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15).materialize())
    );
}

#[test]
fn to_async_error16() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<i32>::OnError(ex.clone())],
        to_enumerable(Observable::try_to_async(move |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32, _h: i32, _i: i32, _j: i32, _k: i32, _l: i32, _m: i32, _n: i32, _o: i32, _p: i32| Err::<i32, ObservableError>(err.clone()))(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16).materialize())
    );
}

#[test]
fn to_async_action0() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(vec![()], to_enumerable(Observable::to_async(move || ran.set(true))()));
    let ran = has_run.clone();
    assert_eq!(vec![()], to_enumerable(Observable::to_async_on(move || ran.set(true), SchedulerDefaults::default_scheduler())()));
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error0() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(vec![Notification::<()>::OnError(ex.clone())], to_enumerable(Observable::try_to_async(move || Err::<(), ObservableError>(err.clone()))().materialize()));
}

#[test]
fn to_async_action1() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32| {
            assert_eq!(1, a);
            ran.set(true);
        })(1))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32| {
                assert_eq!(1, a);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error1() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32| {
                assert_eq!(1, a);
                Err::<(), ObservableError>(err.clone())
            })(1)
            .materialize()
        )
    );
}

#[test]
fn to_async_action2() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            ran.set(true);
        })(1, 2))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error2() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                Err::<(), ObservableError>(err.clone())
            })(1, 2)
            .materialize()
        )
    );
}

#[test]
fn to_async_action3() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            ran.set(true);
        })(1, 2, 3))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error3() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3)
            .materialize()
        )
    );
}

#[test]
fn to_async_action4() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            ran.set(true);
        })(1, 2, 3, 4))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error4() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4)
            .materialize()
        )
    );
}

#[test]
fn to_async_action5() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            ran.set(true);
        })(1, 2, 3, 4, 5))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error5() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5)
            .materialize()
        )
    );
}

#[test]
fn to_async_action6() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error6() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6)
            .materialize()
        )
    );
}

#[test]
fn to_async_action7() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error7() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7)
            .materialize()
        )
    );
}

#[test]
fn to_async_action8() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            assert_eq!(8, h);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7, 8))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7, 8))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error8() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7, 8)
            .materialize()
        )
    );
}

#[test]
fn to_async_action9() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            assert_eq!(8, h);
            assert_eq!(9, i);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7, 8, 9))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7, 8, 9))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error9() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7, 8, 9)
            .materialize()
        )
    );
}

#[test]
fn to_async_action10() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            assert_eq!(8, h);
            assert_eq!(9, i);
            assert_eq!(10, j);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7, 8, 9, 10))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error10() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
            .materialize()
        )
    );
}

#[test]
fn to_async_action11() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            assert_eq!(8, h);
            assert_eq!(9, i);
            assert_eq!(10, j);
            assert_eq!(11, k);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error11() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)
            .materialize()
        )
    );
}

#[test]
fn to_async_action12() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            assert_eq!(8, h);
            assert_eq!(9, i);
            assert_eq!(10, j);
            assert_eq!(11, k);
            assert_eq!(12, l);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error12() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)
            .materialize()
        )
    );
}

#[test]
fn to_async_action13() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            assert_eq!(8, h);
            assert_eq!(9, i);
            assert_eq!(10, j);
            assert_eq!(11, k);
            assert_eq!(12, l);
            assert_eq!(13, m);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                assert_eq!(13, m);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error13() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                assert_eq!(13, m);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)
            .materialize()
        )
    );
}

#[test]
fn to_async_action14() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            assert_eq!(8, h);
            assert_eq!(9, i);
            assert_eq!(10, j);
            assert_eq!(11, k);
            assert_eq!(12, l);
            assert_eq!(13, m);
            assert_eq!(14, n);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                assert_eq!(13, m);
                assert_eq!(14, n);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error14() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                assert_eq!(13, m);
                assert_eq!(14, n);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14)
            .materialize()
        )
    );
}

#[test]
fn to_async_action15() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            assert_eq!(8, h);
            assert_eq!(9, i);
            assert_eq!(10, j);
            assert_eq!(11, k);
            assert_eq!(12, l);
            assert_eq!(13, m);
            assert_eq!(14, n);
            assert_eq!(15, o);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                assert_eq!(13, m);
                assert_eq!(14, n);
                assert_eq!(15, o);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error15() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                assert_eq!(13, m);
                assert_eq!(14, n);
                assert_eq!(15, o);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15)
            .materialize()
        )
    );
}

#[test]
fn to_async_action16() {
    let has_run = Rc::new(Cell::new(false));
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32, p: i32| {
            assert_eq!(1, a);
            assert_eq!(2, b);
            assert_eq!(3, c);
            assert_eq!(4, d);
            assert_eq!(5, e);
            assert_eq!(6, f);
            assert_eq!(7, g);
            assert_eq!(8, h);
            assert_eq!(9, i);
            assert_eq!(10, j);
            assert_eq!(11, k);
            assert_eq!(12, l);
            assert_eq!(13, m);
            assert_eq!(14, n);
            assert_eq!(15, o);
            assert_eq!(16, p);
            ran.set(true);
        })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16))
    );
    let ran = has_run.clone();
    assert_eq!(
        vec![()],
        to_enumerable(Observable::to_async_on(
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32, p: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                assert_eq!(13, m);
                assert_eq!(14, n);
                assert_eq!(15, o);
                assert_eq!(16, p);
                ran.set(true);
            },
            SchedulerDefaults::default_scheduler()
        )(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16))
    );
    assert!(has_run.get(), "has run");
}

#[test]
fn to_async_action_error16() {
    let ex = new_error("ex");
    let err = ex.clone();
    assert_eq!(
        vec![Notification::<()>::OnError(ex.clone())],
        to_enumerable(
            Observable::try_to_async(move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32, j: i32, k: i32, l: i32, m: i32, n: i32, o: i32, p: i32| {
                assert_eq!(1, a);
                assert_eq!(2, b);
                assert_eq!(3, c);
                assert_eq!(4, d);
                assert_eq!(5, e);
                assert_eq!(6, f);
                assert_eq!(7, g);
                assert_eq!(8, h);
                assert_eq!(9, i);
                assert_eq!(10, j);
                assert_eq!(11, k);
                assert_eq!(12, l);
                assert_eq!(13, m);
                assert_eq!(14, n);
                assert_eq!(15, o);
                assert_eq!(16, p);
                Err::<(), ObservableError>(err.clone())
            })(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)
            .materialize()
        )
    );
}

// Not ported: ToAsync_ArgumentChecking - argument validation
