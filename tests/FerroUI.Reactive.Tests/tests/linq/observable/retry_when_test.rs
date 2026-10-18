//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RetryWhenTest.cs` of Rx.NET.
//! Adapted: a handler that throws is the `try_` form returning the error.

use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::rc::Rc;

#[test]
fn retry_when_observable_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_completed(250)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().retry_when(|v| v)
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_completed(450));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn retry_when_observable_handler_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let res = scheduler.start({
        let ex = ex.clone();
        move || Observable::return_(1).try_retry_when(move |_| Err::<Rc<dyn IObservable<i32>>, _>(ex.clone()))
    });

    assert_messages!(res.messages(), RT::on_error(200, ex.clone()));
}

// Not ported: RetryWhen_Observable_ArgumentChecking - argument validation
// Not ported: RetryWhen_Observable_Handler_Completes - needs operators Take and Skip (phase 3, later)
// Not ported: RetryWhen_Observable_Handler_Errors - needs operator SelectMany (phase 3, later)
// Not ported: RetryWhen_Observable_RetryCount_Basic - needs operator SelectMany (phase 3, later)
// Not ported: RetryWhen_Observable_RetryCount_Delayed - needs operators SelectMany and Delay (phase 3, later)
