//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/TimestampTest.cs` of Rx.NET.
//! Adapted: `new DateTimeOffset(ticks, TimeSpan.Zero)` is the time of the test scheduler at those
//! ticks (`ticks_to_time`).

use ferroui_reactive::{ObservableExt, Timestamped};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_time, ReactiveTest as RT, TestScheduler};

#[test]
fn timestamp_regular() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(230, 3),
        RT::on_next(260, 4),
        RT::on_next(300, 5),
        RT::on_next(350, 6),
        RT::on_completed(400),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timestamp_on(s));

    assert_messages!(
        res.messages(),
        RT::on_next(210, Timestamped::new(2, ticks_to_time(210))),
        RT::on_next(230, Timestamped::new(3, ticks_to_time(230))),
        RT::on_next(260, Timestamped::new(4, ticks_to_time(260))),
        RT::on_next(300, Timestamped::new(5, ticks_to_time(300))),
        RT::on_next(350, Timestamped::new(6, ticks_to_time(350))),
        RT::on_completed::<Timestamped<i32>>(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn timestamp_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(400)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timestamp_on(s));

    assert_messages!(res.messages(), RT::on_completed::<Timestamped<i32>>(400));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn timestamp_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(400, ex.clone())]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timestamp_on(s));

    assert_messages!(res.messages(), RT::on_error::<Timestamped<i32>>(400, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn timestamp_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timestamp_on(s));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

// Not ported: Timestamp_ArgumentChecking - argument validation
// Not ported: Timestamp_DefaultScheduler - blocking operator First (waived: one thread)
