//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DelaySubscriptionTest.cs` of Rx.NET.
//! Adapted: `TimeSpan.FromTicks` is `ticks_to_duration` and `new DateTimeOffset(ticks,
//! TimeSpan.Zero)` the time of the test scheduler at those ticks (`ticks_to_time`).

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{
    assert_messages, assert_subscriptions, new_error, ticks_to_duration, ticks_to_time, ReactiveTest as RT, TestScheduler,
};

#[test]
fn delay_subscription_time_span_simple() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(50, 42), RT::on_next(60, 43), RT::on_completed(70)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.delay_subscription_on(ticks_to_duration(30), s));

    assert_messages!(res.messages(), RT::on_next(280, 42), RT::on_next(290, 43), RT::on_completed::<i32>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(230, 300));
}

#[test]
fn delay_subscription_time_span_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(50, 42), RT::on_next(60, 43), RT::on_error(70, ex.clone())]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.delay_subscription_on(ticks_to_duration(30), s));

    assert_messages!(res.messages(), RT::on_next(280, 42), RT::on_next(290, 43), RT::on_error::<i32>(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(230, 300));
}

#[test]
fn delay_subscription_date_time_offset_simple() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(50, 42), RT::on_next(60, 43), RT::on_completed(70)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.delay_subscription_at_on(ticks_to_time(230), s));

    assert_messages!(res.messages(), RT::on_next(280, 42), RT::on_next(290, 43), RT::on_completed::<i32>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(230, 300));
}

#[test]
fn delay_subscription_date_time_offset_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(50, 42), RT::on_next(60, 43), RT::on_error(70, ex.clone())]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.delay_subscription_at_on(ticks_to_time(230), s));

    assert_messages!(res.messages(), RT::on_next(280, 42), RT::on_next(290, 43), RT::on_error::<i32>(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(230, 300));
}

// Not ported: DelaySubscription_ArgumentChecking - argument validation (and a negative TimeSpan, which a Duration cannot be)
// Not ported: DelaySubscription_TimeSpan_Default - needs operator ForEach (the blocking operators, phase 3, later)
// Not ported: DelaySubscription_DateTimeOffset_Default - needs operator ForEach (the blocking operators, phase 3, later)
