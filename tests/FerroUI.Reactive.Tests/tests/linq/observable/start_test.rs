//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/StartTest.cs` of Rx.NET.
//! Adapted: `Start(Action)` is `Observable::start_action_on`, `Start(Func<T>)` is
//! `Observable::start_on`; the flag the action sets is a shared `Cell`.

use ferroui_reactive::Observable;
use ferroui_reactive_testing::{assert_messages, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

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
fn start_func2() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::start_on(|| 1, s));

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_completed::<i32>(200));
}

// Not ported: Start_ArgumentChecking - argument validation
// Not ported: Start_Action - real threads (the default scheduler) and needs operator ToEnumerable (phase 3, later)
// Not ported: Start_ActionError - real threads (the default scheduler) and needs operators Materialize and ToEnumerable (phase 3, later)
// Not ported: Start_ActionErrorAfterUnsubscribeDoesNotThrow - real threads (Barrier)
// Not ported: Start_Func - real threads (the default scheduler) and needs operator ToEnumerable (phase 3, later)
// Not ported: Start_FuncError - real threads (the default scheduler) and needs operators Materialize and ToEnumerable (phase 3, later)
