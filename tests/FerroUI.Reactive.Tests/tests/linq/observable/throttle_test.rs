//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ThrottleTest.cs` of Rx.NET.
//! Adapted: `TimeSpan.FromTicks` is `ticks_to_duration`; the throttle duration selector receives
//! the value by reference and returns the sequence as a handle; a selector that throws is the
//! `try_` form returning `Err`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};

#[test]
fn throttle_time_span_all_pass() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 0),
        RT::on_next(210, 1),
        RT::on_next(240, 2),
        RT::on_next(270, 3),
        RT::on_next(300, 4),
        RT::on_completed(400),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.throttle_on(ticks_to_duration(20), s));

    assert_messages!(
        res.messages(),
        RT::on_next(230, 1),
        RT::on_next(260, 2),
        RT::on_next(290, 3),
        RT::on_next(320, 4),
        RT::on_completed::<i32>(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn throttle_time_span_all_pass_error_end() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 0),
        RT::on_next(210, 1),
        RT::on_next(240, 2),
        RT::on_next(270, 3),
        RT::on_next(300, 4),
        RT::on_error(400, ex.clone()),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.throttle_on(ticks_to_duration(20), s));

    assert_messages!(
        res.messages(),
        RT::on_next(230, 1),
        RT::on_next(260, 2),
        RT::on_next(290, 3),
        RT::on_next(320, 4),
        RT::on_error::<i32>(400, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn throttle_time_span_all_drop() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 0),
        RT::on_next(210, 1),
        RT::on_next(240, 2),
        RT::on_next(270, 3),
        RT::on_next(300, 4),
        RT::on_next(330, 5),
        RT::on_next(360, 6),
        RT::on_next(390, 7),
        RT::on_completed(400),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.throttle_on(ticks_to_duration(40), s));

    assert_messages!(res.messages(), RT::on_next(400, 7), RT::on_completed::<i32>(400));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn throttle_time_span_all_drop_error_end() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 0),
        RT::on_next(210, 1),
        RT::on_next(240, 2),
        RT::on_next(270, 3),
        RT::on_next(300, 4),
        RT::on_next(330, 5),
        RT::on_next(360, 6),
        RT::on_next(390, 7),
        RT::on_error(400, ex.clone()),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.throttle_on(ticks_to_duration(40), s));

    assert_messages!(res.messages(), RT::on_error::<i32>(400, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn throttle_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 0), RT::on_completed(300)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.throttle_on(ticks_to_duration(10), s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn throttle_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 0), RT::on_error(300, ex.clone())]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.throttle_on(ticks_to_duration(10), s));

    assert_messages!(res.messages(), RT::on_error::<i32>(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn throttle_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 0)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.throttle_on(ticks_to_duration(10), s));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn throttle_simple() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 0),
        RT::on_next(210, 1),
        RT::on_next(240, 2),
        RT::on_next(250, 3),
        RT::on_next(280, 4),
        RT::on_completed(300),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.throttle_on(ticks_to_duration(20), s));

    assert_messages!(res.messages(), RT::on_next(230, 1), RT::on_next(270, 3), RT::on_next(300, 4), RT::on_completed::<i32>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn throttle_duration_delay_behavior() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, -1),
        RT::on_next(250, 0),
        RT::on_next(280, 1),
        RT::on_next(310, 2),
        RT::on_next(350, 3),
        RT::on_next(400, 4),
        RT::on_completed(550),
    ]);

    let ys = vec![
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
    ];

    let (source, durations) = (xs.as_observable(), ys.clone());
    let res = scheduler.start(move || source.throttle_by(move |x: &i32| durations[*x as usize].as_observable()));

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 20, 0),
        RT::on_next(280 + 20, 1),
        RT::on_next(310 + 20, 2),
        RT::on_next(350 + 20, 3),
        RT::on_next(400 + 20, 4),
        RT::on_completed::<i32>(550),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));

    assert_subscriptions!(ys[0].subscriptions(), RT::subscribe(250, 250 + 20));
    assert_subscriptions!(ys[1].subscriptions(), RT::subscribe(280, 280 + 20));
    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(310, 310 + 20));
    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(350, 350 + 20));
    assert_subscriptions!(ys[4].subscriptions(), RT::subscribe(400, 400 + 20));
}

#[test]
fn throttle_duration_throttle_behavior() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, -1),
        RT::on_next(250, 0),
        RT::on_next(280, 1),
        RT::on_next(310, 2),
        RT::on_next(350, 3),
        RT::on_next(400, 4),
        RT::on_completed(550),
    ]);

    let ys = vec![
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(40, 42), RT::on_next(45, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(60, 42), RT::on_next(65, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
    ];

    let (source, durations) = (xs.as_observable(), ys.clone());
    let res = scheduler.start(move || source.throttle_by(move |x: &i32| durations[*x as usize].as_observable()));

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 20, 0),
        RT::on_next(310 + 20, 2),
        RT::on_next(400 + 20, 4),
        RT::on_completed::<i32>(550),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));

    assert_subscriptions!(ys[0].subscriptions(), RT::subscribe(250, 250 + 20));
    assert_subscriptions!(ys[1].subscriptions(), RT::subscribe(280, 310));
    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(310, 310 + 20));
    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(350, 400));
    assert_subscriptions!(ys[4].subscriptions(), RT::subscribe(400, 400 + 20));
}

#[test]
fn throttle_duration_early_completion() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, -1),
        RT::on_next(250, 0),
        RT::on_next(280, 1),
        RT::on_next(310, 2),
        RT::on_next(350, 3),
        RT::on_next(400, 4),
        RT::on_completed(410),
    ]);

    let ys = vec![
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(40, 42), RT::on_next(45, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(60, 42), RT::on_next(65, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
    ];

    let (source, durations) = (xs.as_observable(), ys.clone());
    let res = scheduler.start(move || source.throttle_by(move |x: &i32| durations[*x as usize].as_observable()));

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 20, 0),
        RT::on_next(310 + 20, 2),
        RT::on_next(410, 4),
        RT::on_completed::<i32>(410),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(ys[0].subscriptions(), RT::subscribe(250, 250 + 20));
    assert_subscriptions!(ys[1].subscriptions(), RT::subscribe(280, 310));
    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(310, 310 + 20));
    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(350, 400));
    assert_subscriptions!(ys[4].subscriptions(), RT::subscribe(400, 410));
}

#[test]
fn throttle_duration_inner_error() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let ex = new_error("ex");

    let (source, sch, error) = (xs.as_observable(), scheduler.clone(), ex.clone());
    let res = scheduler.start(move || {
        source.throttle_by(move |x: &i32| {
            let x = *x as i64;
            if x < 4 {
                sch.create_cold_observable(vec![RT::on_next(x * 10, "Ignore"), RT::on_next(x * 10 + 5, "Aargh!")]).as_observable()
            } else {
                sch.create_cold_observable(vec![RT::on_error::<&str>(x * 10, error.clone())]).as_observable()
            }
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_error::<i32>(450 + 4 * 10, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 490));
}

#[test]
fn throttle_duration_outer_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_error(460, ex.clone()),
    ]);

    let (source, sch) = (xs.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.throttle_by(move |x: &i32| {
            let x = *x as i64;
            sch.create_cold_observable(vec![RT::on_next(x * 10, "Ignore"), RT::on_next(x * 10 + 5, "Aargh!")]).as_observable()
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_error::<i32>(460, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 460));
}

#[test]
fn throttle_duration_selector_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let ex = new_error("ex");

    let (source, sch, error) = (xs.as_observable(), scheduler.clone(), ex.clone());
    let res = scheduler.start(move || {
        source.try_throttle_by(move |x: &i32| {
            let x = *x as i64;
            if x < 4 {
                Ok(sch.create_cold_observable(vec![RT::on_next(x * 10, "Ignore"), RT::on_next(x * 10 + 5, "Aargh!")]).as_observable())
            } else {
                Err(error.clone())
            }
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_error::<i32>(450, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn throttle_duration_inner_done_delay_behavior() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let (source, sch) = (xs.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.throttle_by(move |x: &i32| sch.create_cold_observable(vec![RT::on_completed::<&str>(*x as i64 * 10)]).as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_next(450 + 4 * 10, 4),
        RT::on_completed::<i32>(550),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn throttle_duration_inner_done_throttle_behavior() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(280, 3),
        RT::on_next(300, 4),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_completed(550),
    ]);

    let (source, sch) = (xs.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.throttle_by(move |x: &i32| sch.create_cold_observable(vec![RT::on_completed::<&str>(*x as i64 * 10)]).as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(300 + 4 * 10, 4),
        RT::on_next(410 + 6 * 10, 6),
        RT::on_completed::<i32>(550),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

// Not ported: Throttle_ArgumentChecking - argument validation (and a negative TimeSpan, which a Duration cannot be)
// Not ported: Throttle_DefaultScheduler - needs operator ToEnumerable (the blocking operators, phase 3, later)
// Not ported: Throttle_Duration_ArgumentChecking - argument validation
