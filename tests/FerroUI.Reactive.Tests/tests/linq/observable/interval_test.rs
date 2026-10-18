//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/IntervalTest.cs` of Rx.NET.
//! Adapted: an observer that throws is one that panics, and the scheduler that runs it panics,
//! checked with `assert_throws`.

use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_throws, ticks_to_duration, ReactiveTest as RT, TestScheduler};

#[test]
fn interval_time_span_basic() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::interval_on(ticks_to_duration(100), s));

    assert_messages!(
        res.messages(),
        RT::on_next(300, 0i64),
        RT::on_next(400, 1i64),
        RT::on_next(500, 2i64),
        RT::on_next(600, 3i64),
        RT::on_next(700, 4i64),
        RT::on_next(800, 5i64),
        RT::on_next(900, 6i64)
    );
}

#[test]
fn interval_time_span_zero() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::interval_on(ticks_to_duration(0), s), 210);

    assert_messages!(
        res.messages(),
        RT::on_next(201, 0i64),
        RT::on_next(202, 1i64),
        RT::on_next(203, 2i64),
        RT::on_next(204, 3i64),
        RT::on_next(205, 4i64),
        RT::on_next(206, 5i64),
        RT::on_next(207, 6i64),
        RT::on_next(208, 7i64),
        RT::on_next(209, 8i64)
    );
}

#[test]
fn interval_time_span_disposed() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::interval_on(ticks_to_duration(1000), s));

    assert_messages!(res.messages());
}

#[test]
fn interval_time_span_observer_throws() {
    let scheduler = TestScheduler::new();

    let xs = Observable::interval_on(ticks_to_duration(1), scheduler.scheduler());

    xs.subscribe_next(|_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler.run());
}

// Not ported: Interval_TimeSpan_ArgumentChecking - argument validation (a null scheduler, a null observer, a negative period: a `Duration` cannot be negative)
// Not ported: Interval_TimeSpan_Zero_DefaultScheduler - real threads (ManualResetEvent, the default scheduler) and needs operator TakeWhile (phase 3, later)
// Not ported: Interval_TimeSpan_DefaultScheduler - real threads (the default scheduler) and needs operators ToEnumerable and Take (phase 3, later)
