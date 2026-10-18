//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/TakeUntilTest.cs` of Rx.NET.
//! The operators are applied to the handle of the test observable (`as_observable`); `TakeUntil(DateTimeOffset, IScheduler)` is `take_until_at_on`, with `new DateTimeOffset(t, TimeSpan.Zero)` as `ticks_to_time(t)`;
//! `TakeUntil(Func<T, bool>)` is `take_until_predicate` (`try_take_until_predicate` for a predicate that throws) and
//! `TakeUntil(CancellationToken)` is `take_until_cancellation`.

use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_time, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::CancellationTokenSource;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn take_until_preempt_some_data_next() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(225, 99),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_completed::<i32>(225),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 225),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn take_until_preempt_some_data_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(225, ex.clone()),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_error::<i32>(225, ex.clone()),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 225),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn take_until_no_preempt_some_data_empty() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(225),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 250),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn take_until_no_preempt_some_data_never() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 250),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 250),
    );
}

#[test]
fn take_until_preempt_never_next() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(225, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(225),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 225),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn take_until_preempt_never_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(225, ex.clone()),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(225, ex.clone()),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 225),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn take_until_no_preempt_never_empty() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(225),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 1000),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn take_until_no_preempt_never_never() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 1000),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn take_until_preempt_before_first_produced() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(230, 2),
        RT::on_completed::<i32>(240),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(220),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(210),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 210),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn take_until_preempt_before_first_produced_remain_silent_and_proper_disposed() {
    let scheduler = TestScheduler::new();

    let source_not_disposed = Rc::new(Cell::new(false));

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        // Should not come.
        RT::on_error::<i32>(215, new_error("ex")),
        RT::on_completed::<i32>(240),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(220),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        let source_not_disposed = source_not_disposed.clone();
        scheduler.start(move || l.as_observable().do_(move |_| source_not_disposed.set(true)).take_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(210),
    );

    assert!(!source_not_disposed.get());
}

#[test]
fn take_until_no_preempt_after_last_produced_proper_disposed_signal() {
    let scheduler = TestScheduler::new();

    let signal_not_disposed = Rc::new(Cell::new(false));

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(230, 2),
        RT::on_completed::<i32>(240),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_completed::<i32>(260),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        let signal_not_disposed = signal_not_disposed.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable().do_(move |_| signal_not_disposed.set(true))))
    };

    assert_messages!(res.messages(),
        RT::on_next(230, 2),
        RT::on_completed::<i32>(240),
    );

    assert!(!signal_not_disposed.get());
}

#[test]
fn take_until_error_some() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(225, ex.clone()),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(240, 2),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().take_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(225, ex.clone()),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 225),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn take_until_immediate() {
    let scheduler = TestScheduler::new();

    let xs = Observable::return_(1);
    let ys = Observable::return_("bar");

    let res = {
        scheduler.start(move || xs.take_until(ys))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(200),
    );
}

#[test]
fn take_until_zero() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_until_at_on(ticks_to_time(0), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(201),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 201),
    );
}

#[test]
fn take_until_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(240),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_until_at_on(ticks_to_time(225), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(225),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn take_until_late() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_until_at_on(ticks_to_time(250), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_until_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_until_at_on(ticks_to_time(250), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn take_until_never() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_until_at_on(ticks_to_time(250), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 250),
    );
}

#[test]
fn take_until_twice1() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_completed::<i32>(270),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_until_at_on(ticks_to_time(255), s.clone()).take_until_at_on(ticks_to_time(235), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(235),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 235),
    );
}

#[test]
fn take_until_twice2() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_completed::<i32>(270),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_until_at_on(ticks_to_time(235), s.clone()).take_until_at_on(ticks_to_time(255), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(235),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 235),
    );
}

#[test]
fn take_until_predicate_basic() {
    let scheduler = TestScheduler::new();

    let source = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_next(60, 6),
        RT::on_next(70, 7),
        RT::on_next(80, 8),
        RT::on_next(90, 9),
        RT::on_completed::<i32>(100),
    ]);

    let result = {
        let source = source.clone();
        scheduler.start(move || source.as_observable().take_until_predicate(|v| *v == 5))
    };

    assert_messages!(result.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(source.subscriptions(),
        RT::subscribe(200, 250),
    );
}

#[test]
fn take_until_predicate_true() {
    let scheduler = TestScheduler::new();

    let source = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_next(60, 6),
        RT::on_next(70, 7),
        RT::on_next(80, 8),
        RT::on_next(90, 9),
        RT::on_completed::<i32>(100),
    ]);

    let result = {
        let source = source.clone();
        scheduler.start(move || source.as_observable().take_until_predicate(|_| true))
    };

    assert_messages!(result.messages(),
        RT::on_next(210, 1),
        RT::on_completed::<i32>(210),
    );

    assert_subscriptions!(source.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn take_until_predicate_false() {
    let scheduler = TestScheduler::new();

    let source = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_next(60, 6),
        RT::on_next(70, 7),
        RT::on_next(80, 8),
        RT::on_next(90, 9),
        RT::on_completed::<i32>(100),
    ]);

    let result = {
        let source = source.clone();
        scheduler.start(move || source.as_observable().take_until_predicate(|_| false))
    };

    assert_messages!(result.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_next(270, 7),
        RT::on_next(280, 8),
        RT::on_next(290, 9),
        RT::on_completed::<i32>(300),
    );

    assert_subscriptions!(source.subscriptions(),
        RT::subscribe(200, 300),
    );
}

#[test]
fn take_until_predicate_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let source = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_error::<i32>(40, ex.clone()),
    ]);

    let result = {
        let source = source.clone();
        scheduler.start(move || source.as_observable().take_until_predicate(|_| false))
    };

    assert_messages!(result.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_error::<i32>(240, ex.clone()),
    );

    assert_subscriptions!(source.subscriptions(),
        RT::subscribe(200, 240),
    );
}

#[test]
fn take_until_predicate_crash() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let source = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_completed::<i32>(260),
    ]);

    let result = {
        let source = source.clone();
        let ex = ex.clone();
        scheduler.start(move || source.as_observable().try_take_until_predicate(move |v| if *v == 3 { Err(ex.clone()) } else { Ok(false) }))
    };

    assert_messages!(result.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_error::<i32>(230, ex.clone()),
    );

    assert_subscriptions!(source.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_until_cancellation_token_basic_cancelation() {
    let scheduler = TestScheduler::new();
    let token_source = CancellationTokenSource::new();

    let source = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_completed::<i32>(260),
    ]);

    {
        let token_source = token_source.clone();
        scheduler.schedule_absolute_action(235, move || token_source.cancel());
    }

    let result = {
        let source = source.clone();
        let token = token_source.token();
        scheduler.start(move || source.as_observable().take_until_cancellation(token))
    };

    assert_messages!(result.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(235),
    );

    assert_subscriptions!(source.subscriptions(),
        RT::subscribe(200, 235),
    );
}

#[test]
fn take_until_cancellation_token_already_canceled() {
    let scheduler = TestScheduler::new();
    let token_source = CancellationTokenSource::new();
    token_source.cancel();

    let source = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_completed::<i32>(260),
    ]);

    let result = {
        let source = source.clone();
        let token = token_source.token();
        scheduler.start(move || source.as_observable().take_until_cancellation(token))
    };

    assert_messages!(result.messages(),
        RT::on_completed::<i32>(200),
    );

    assert!(source.subscriptions().is_empty());
}

// Not ported: TakeUntil_ArgumentChecking - argument validation
// Not ported: TakeUntil_Timed_ArgumentChecking - argument validation
// Not ported: TakeUntil_Default - real threads
// Not ported: TakeUntil_Predicate_ArgumentChecking - argument validation
