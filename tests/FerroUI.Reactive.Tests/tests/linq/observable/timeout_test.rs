//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/TimeoutTest.cs` of Rx.NET.
//! Adapted: `TimeSpan.FromTicks` is `ticks_to_duration` and `new DateTimeOffset(new DateTime(ticks),
//! TimeSpan.Zero)` the time of the test scheduler at those ticks (`ticks_to_time`);
//! `ex is TimeoutException` is `e.is::<TimeoutException>()`. The overloads are `timeout_on`
//! (`Timeout(TimeSpan, IScheduler)`), `timeout_with_on` (with another sequence), `timeout_at_on` and
//! `timeout_at_with_on` (`DateTimeOffset`), `timeout_by` and `timeout_by_with` (a timeout selector),
//! `timeout_by_first` and `timeout_by_first_with` (a first timeout and a selector); a selector that
//! throws is the `try_` form returning `Err`.

use ferroui_reactive::linq::TimeoutException;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{
    assert_messages, assert_subscriptions, new_error, ticks_to_duration, ticks_to_time, ReactiveTest as RT, TestScheduler,
};

#[test]
fn timeout_in_time() {
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
    let res = scheduler.start(move || source.timeout_on(ticks_to_duration(500), s));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(230, 3),
        RT::on_next(260, 4),
        RT::on_next(300, 5),
        RT::on_next(350, 6),
        RT::on_completed::<i32>(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn timeout_date_time_offset_timeout_occurs_with_default_exception() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(410, 1)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_at_on(ticks_to_time(400), s));

    assert_messages!(res.messages(), RT::on_error_where::<i32>(400, |ex| ex.is::<TimeoutException>()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn timeout_time_span_timeout_occurs_with_default_exception() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(410, 1)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_on(ticks_to_duration(200), s));

    assert_messages!(res.messages(), RT::on_error_where::<i32>(400, |ex| ex.is::<TimeoutException>()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn timeout_timeout_occurs_1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(130, 2),
        RT::on_next(310, 3),
        RT::on_next(400, 4),
        RT::on_completed(500),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(50, -1), RT::on_next(200, -2), RT::on_next(310, -3), RT::on_completed(320)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_with_on(ticks_to_duration(100), other, s));

    assert_messages!(res.messages(), RT::on_next(350, -1), RT::on_next(500, -2), RT::on_next(610, -3), RT::on_completed::<i32>(620));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 620));
}

#[test]
fn timeout_timeout_occurs_2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(130, 2),
        RT::on_next(240, 3),
        RT::on_next(310, 4),
        RT::on_next(430, 5),
        RT::on_completed(500),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(50, -1), RT::on_next(200, -2), RT::on_next(310, -3), RT::on_completed(320)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_with_on(ticks_to_duration(100), other, s));

    assert_messages!(
        res.messages(),
        RT::on_next(240, 3),
        RT::on_next(310, 4),
        RT::on_next(460, -1),
        RT::on_next(610, -2),
        RT::on_next(720, -3),
        RT::on_completed::<i32>(730),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(410, 730));
}

#[test]
fn timeout_timeout_occurs_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(130, 2),
        RT::on_next(240, 3),
        RT::on_next(310, 4),
        RT::on_next(430, 5),
        RT::on_completed(500),
    ]);

    let ys = scheduler.create_cold_observable::<i32>(vec![]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_with_on(ticks_to_duration(100), other, s));

    assert_messages!(res.messages(), RT::on_next(240, 3), RT::on_next(310, 4));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(410, 1000));
}

#[test]
fn timeout_timeout_occurs_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(500)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, -1)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_with_on(ticks_to_duration(100), other, s));

    assert_messages!(res.messages(), RT::on_next(400, -1));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 1000));
}

#[test]
fn timeout_timeout_occurs_error() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_error::<i32>(500, new_error("ex"))]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, -1)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_with_on(ticks_to_duration(100), other, s));

    assert_messages!(res.messages(), RT::on_next(400, -1));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 1000));
}

#[test]
fn timeout_timeout_occurs_next_is_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(500, 42)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_error::<i32>(100, ex.clone())]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_with_on(ticks_to_duration(100), other, s));

    assert_messages!(res.messages(), RT::on_error::<i32>(400, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 400));
}

#[test]
fn timeout_timeout_not_occurs_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(250)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, -1)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_with_on(ticks_to_duration(100), other, s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn timeout_timeout_not_occurs_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_error::<i32>(250, ex.clone())]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, -1)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_with_on(ticks_to_duration(100), other, s));

    assert_messages!(res.messages(), RT::on_error::<i32>(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn timeout_timeout_does_not_occur() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(130, 2),
        RT::on_next(240, 3),
        RT::on_next(320, 4),
        RT::on_next(410, 5),
        RT::on_completed(500),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(50, -1), RT::on_next(200, -2), RT::on_next(310, -3), RT::on_completed(320)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_with_on(ticks_to_duration(100), other, s));

    assert_messages!(res.messages(), RT::on_next(240, 3), RT::on_next(320, 4), RT::on_next(410, 5), RT::on_completed::<i32>(500));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 500));

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn timeout_date_time_offset_timeout_occurs() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(410, 1)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, -1)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_at_with_on(ticks_to_time(400), other, s));

    assert_messages!(res.messages(), RT::on_next(500, -1));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(400, 1000));
}

#[test]
fn timeout_date_time_offset_timeout_does_not_occur_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_completed(390)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, -1)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_at_with_on(ticks_to_time(400), other, s));

    assert_messages!(res.messages(), RT::on_next(310, 1), RT::on_completed::<i32>(390));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn timeout_date_time_offset_timeout_does_not_occur_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_error(390, ex.clone())]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, -1)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_at_with_on(ticks_to_time(400), other, s));

    assert_messages!(res.messages(), RT::on_next(310, 1), RT::on_error::<i32>(390, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn timeout_date_time_offset_timeout_occur_2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, -1)]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_at_with_on(ticks_to_time(400), other, s));

    assert_messages!(res.messages(), RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(500, -1));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(400, 1000));
}

#[test]
fn timeout_date_time_offset_timeout_occur_3() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable::<i32>(vec![]);

    let (source, other, s) = (xs.as_observable(), ys.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.timeout_at_with_on(ticks_to_time(400), other, s));

    assert_messages!(res.messages(), RT::on_next(310, 1), RT::on_next(350, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(400, 1000));
}

#[test]
fn timeout_duration_simple_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable::<i32>(vec![]);

    let (source, first, timeouts) = (xs.as_observable(), ys.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.timeout_by_first(first, move |_: &i32| timeouts.clone()));

    assert_messages!(res.messages(), RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed::<i32>(450));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(200, 310),
        RT::subscribe(310, 350),
        RT::subscribe(350, 420),
        RT::subscribe(420, 450),
    );
}

#[test]
fn timeout_duration_simple_timeout_first() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, "Boo!")]);

    let zs = scheduler.create_cold_observable::<&str>(vec![]);

    let (source, first, timeouts) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || source.timeout_by_first(first, move |_: &i32| timeouts.clone()));

    assert_messages!(res.messages(), RT::on_error_where::<i32>(300, |ex| ex.is::<TimeoutException>()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(zs.subscriptions());
}

#[test]
fn timeout_duration_simple_timeout_first_other() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, "Boo!")]);

    let zs = scheduler.create_cold_observable::<&str>(vec![]);

    let ts = scheduler.create_cold_observable(vec![RT::on_next(50, 42), RT::on_completed(70)]);

    let (source, first, timeouts, other) = (xs.as_observable(), ys.as_observable(), zs.as_observable(), ts.as_observable());
    let res = scheduler.start(move || source.timeout_by_first_with(first, move |_: &i32| timeouts.clone(), other));

    assert_messages!(res.messages(), RT::on_next(350, 42), RT::on_completed::<i32>(370));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(zs.subscriptions());

    assert_subscriptions!(ts.subscriptions(), RT::subscribe(300, 370));
}

#[test]
fn timeout_duration_simple_timeout_later() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable::<&str>(vec![]);

    let zs = scheduler.create_cold_observable(vec![RT::on_next(50, "Boo!")]);

    let (source, first, timeouts) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || source.timeout_by_first(first, move |_: &i32| timeouts.clone()));

    assert_messages!(
        res.messages(),
        RT::on_next(310, 1),
        RT::on_next(350, 2),
        RT::on_error_where::<i32>(400, |ex| ex.is::<TimeoutException>()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(310, 350), RT::subscribe(350, 400));
}

#[test]
fn timeout_duration_simple_timeout_later_other() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable::<&str>(vec![]);

    let zs = scheduler.create_cold_observable(vec![RT::on_next(50, "Boo!")]);

    let ts = scheduler.create_cold_observable(vec![RT::on_next(50, 42), RT::on_completed(70)]);

    let (source, first, timeouts, other) = (xs.as_observable(), ys.as_observable(), zs.as_observable(), ts.as_observable());
    let res = scheduler.start(move || source.timeout_by_first_with(first, move |_: &i32| timeouts.clone(), other));

    assert_messages!(res.messages(), RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(450, 42), RT::on_completed::<i32>(470));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(310, 350), RT::subscribe(350, 400));

    assert_subscriptions!(ts.subscriptions(), RT::subscribe(400, 470));
}

#[test]
fn timeout_duration_simple_timeout_later_no_first() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let zs = scheduler.create_cold_observable(vec![RT::on_next(50, "Boo!")]);

    let (source, timeouts) = (xs.as_observable(), zs.as_observable());
    let res = scheduler.start(move || source.timeout_by(move |_: &i32| timeouts.clone()));

    assert_messages!(
        res.messages(),
        RT::on_next(310, 1),
        RT::on_next(350, 2),
        RT::on_error_where::<i32>(400, |ex| ex.is::<TimeoutException>()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(310, 350), RT::subscribe(350, 400));
}

#[test]
fn timeout_duration_simple_timeout_later_other_no_first() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let zs = scheduler.create_cold_observable(vec![RT::on_next(50, "Boo!")]);

    let ts = scheduler.create_cold_observable(vec![RT::on_next(50, 42), RT::on_completed(70)]);

    let (source, timeouts, other) = (xs.as_observable(), zs.as_observable(), ts.as_observable());
    let res = scheduler.start(move || source.timeout_by_with(move |_: &i32| timeouts.clone(), other));

    assert_messages!(res.messages(), RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(450, 42), RT::on_completed::<i32>(470));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(310, 350), RT::subscribe(350, 400));

    assert_subscriptions!(ts.subscriptions(), RT::subscribe(400, 470));
}

#[test]
fn timeout_duration_simple_timeout_by_completion() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable::<&str>(vec![]);

    let zs = scheduler.create_cold_observable(vec![RT::on_completed::<&str>(50)]);

    let (source, first, timeouts) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || source.timeout_by_first(first, move |_: &i32| timeouts.clone()));

    assert_messages!(
        res.messages(),
        RT::on_next(310, 1),
        RT::on_next(350, 2),
        RT::on_error_where::<i32>(400, |ex| ex.is::<TimeoutException>()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(310, 350), RT::subscribe(350, 400));
}

#[test]
fn timeout_duration_simple_selector_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable::<&str>(vec![]);

    let zs = scheduler.create_cold_observable::<&str>(vec![]);

    let ex = new_error("ex");

    let (source, first, timeouts, error) = (xs.as_observable(), ys.as_observable(), zs.as_observable(), ex.clone());
    let res = scheduler.start(move || {
        source.try_timeout_by_first(first, move |x: &i32| if *x < 3 { Ok(timeouts.clone()) } else { Err(error.clone()) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, 1),
        RT::on_next(350, 2),
        RT::on_next(420, 3),
        RT::on_error::<i32>(420, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(310, 350), RT::subscribe(350, 420));
}

#[test]
fn timeout_duration_simple_inner_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable::<&str>(vec![]);

    let zs = scheduler.create_cold_observable(vec![RT::on_error::<&str>(50, ex.clone())]);

    let (source, first, timeouts) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || source.timeout_by_first(first, move |_: &i32| timeouts.clone()));

    assert_messages!(res.messages(), RT::on_next(310, 1), RT::on_next(350, 2), RT::on_error::<i32>(400, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(310, 350), RT::subscribe(350, 400));
}

#[test]
fn timeout_duration_simple_first_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_completed(450)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_error::<&str>(50, ex.clone())]);

    let zs = scheduler.create_cold_observable::<&str>(vec![]);

    let (source, first, timeouts) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || source.timeout_by_first(first, move |_: &i32| timeouts.clone()));

    assert_messages!(res.messages(), RT::on_error::<i32>(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(zs.subscriptions());
}

#[test]
fn timeout_duration_simple_source_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(310, 1), RT::on_next(350, 2), RT::on_next(420, 3), RT::on_error(450, ex.clone())]);

    let ys = scheduler.create_cold_observable::<&str>(vec![]);

    let zs = scheduler.create_cold_observable::<&str>(vec![]);

    let (source, first, timeouts) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || source.timeout_by_first(first, move |_: &i32| timeouts.clone()));

    assert_messages!(
        res.messages(),
        RT::on_next(310, 1),
        RT::on_next(350, 2),
        RT::on_next(420, 3),
        RT::on_error::<i32>(450, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(310, 350), RT::subscribe(350, 420), RT::subscribe(420, 450));
}

// Not ported: Timeout_ArgumentChecking - argument validation (and a negative TimeSpan, which a Duration cannot be)
// Not ported: Timeout_TimeSpan_DefaultScheduler - blocking operator ToEnumerable (waived: one thread)
// Not ported: Timeout_TimeSpan_Observable_DefaultScheduler - blocking operator ToEnumerable (waived: one thread)
// Not ported: Timeout_DateTimeOffset_DefaultScheduler - blocking operator ToEnumerable (waived: one thread)
// Not ported: Timeout_DateTimeOffset_Observable_DefaultScheduler - blocking operator ToEnumerable (waived: one thread)
// Not ported: Timeout_Duration_ArgumentChecking - argument validation
