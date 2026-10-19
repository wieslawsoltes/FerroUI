//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/StartTest.cs` of Rx.NET.
//! Adapted: `Start(Action)` is `Observable::start_action_on`, `Start(Func<T>)` is
//! `Observable::start_on`, and the forms that throw are the `try_` methods; the flag the action
//! sets is a shared `Cell`. The scheduler of asynchronous conversions, which upstream's
//! `Start(Action)` and `Start(Func<T>)` use, is the default scheduler of the thread (the
//! current-thread scheduler in a test) where upstream has the thread pool, so the function runs
//! before `start` returns; the blocking `ToEnumerable()` is `block_on` of `to_list().to_task()`.

use ferroui_reactive::{Notification, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, new_error, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::tasks::block_on;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn start_action() {
    let done = Rc::new(Cell::new(false));
    let d = done.clone();
    assert_eq!(vec![()], block_on(Observable::start_action(move || d.set(true)).to_list().to_task()).unwrap());
    assert!(done.get(), "done");
}

#[test]
fn start_action2() {
    let scheduler = TestScheduler::new();

    let done = Rc::new(Cell::new(false));

    let (s, d) = (scheduler.scheduler(), done.clone());
    let res = scheduler.start(move || Observable::start_action_on(move || d.set(true), s));

    assert_messages!(res.messages(), RT::on_next(200, ()), RT::on_completed::<()>(200));

    assert!(done.get(), "done");
}

#[test]
fn start_action_error() {
    let ex = new_error("ex");

    let e = ex.clone();
    let res = block_on(Observable::try_start_action(move || Err(e)).materialize().to_list().to_task()).unwrap();

    assert_eq!(vec![Notification::<()>::OnError(ex.clone())], res);
}

#[test]
fn start_func() {
    let res = block_on(Observable::start(|| 1).to_list().to_task()).unwrap();

    assert_eq!(vec![1], res);
}

#[test]
fn start_func2() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::start_on(|| 1, s));

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_completed::<i32>(200));
}

#[test]
fn start_func_error() {
    let ex = new_error("ex");

    let e = ex.clone();
    let res = block_on(Observable::try_start(move || Err::<i32, _>(e)).materialize().to_list().to_task()).unwrap();

    assert_eq!(vec![Notification::<i32>::OnError(ex.clone())], res);
}

// Not ported: Start_ArgumentChecking - argument validation
// Not ported: Start_ActionErrorAfterUnsubscribeDoesNotThrow - real threads (Barrier)
