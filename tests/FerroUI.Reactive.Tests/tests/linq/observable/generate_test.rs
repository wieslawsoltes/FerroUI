//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/GenerateTest.cs` of Rx.NET.
//! Adapted: a function that throws is a function of `Observable::try_generate_on` (and of the timed
//! forms `try_generate_after_on`, `try_generate_at_on`) returning the error; the overloads with a
//! `TimeSpan` selector are `generate_after_on`, those with a `DateTimeOffset` selector
//! `generate_at_on`.

use ferroui_reactive::Observable;
use ferroui_reactive_testing::{assert_messages, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};

// Non-timed

#[test]
fn generate_finite() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::generate_on(0, |x: &i32| *x <= 3, |x: i32| x + 1, |x: &i32| *x, s));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 0),
        RT::on_next(202, 1),
        RT::on_next(203, 2),
        RT::on_next(204, 3),
        RT::on_completed::<i32>(205)
    );
}

#[test]
fn generate_throw_condition() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || Observable::try_generate_on(0, move |_: &i32| Err(error.clone()), |x: i32| Ok(x + 1), |x: &i32| Ok(*x), s));

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn generate_throw_result_selector() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_generate_on(0, |_: &i32| Ok(true), |x: i32| Ok(x + 1), move |_: &i32| -> Result<i32, _> { Err(error.clone()) }, s)
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn generate_throw_iterate() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || Observable::try_generate_on(0, |_: &i32| Ok(true), move |_: i32| Err(error.clone()), |x: &i32| Ok(*x), s));

    assert_messages!(res.messages(), RT::on_next(201, 0), RT::on_error::<i32>(202, ex));
}

#[test]
fn generate_dispose() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::generate_on(0, |_: &i32| true, |x: i32| x + 1, |x: &i32| *x, s), 203);

    assert_messages!(res.messages(), RT::on_next(201, 0), RT::on_next(202, 1));
}

// Timed

#[test]
fn generate_time_span_finite() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || {
        Observable::generate_after_on(0, |x: &i32| *x <= 3, |x: i32| x + 1, |x: &i32| *x, |x: &i32| ticks_to_duration(*x as i64 + 1), s)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(202, 0),
        RT::on_next(204, 1),
        RT::on_next(207, 2),
        RT::on_next(211, 3),
        RT::on_completed::<i32>(211)
    );
}

#[test]
fn generate_time_span_throw_condition() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_generate_after_on(
            0,
            move |_: &i32| Err(error.clone()),
            |x: i32| Ok(x + 1),
            |x: &i32| Ok(*x),
            |x: &i32| Ok(ticks_to_duration(*x as i64 + 1)),
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn generate_time_span_throw_result_selector() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_generate_after_on(
            0,
            |_: &i32| Ok(true),
            |x: i32| Ok(x + 1),
            move |_: &i32| -> Result<i32, _> { Err(error.clone()) },
            |x: &i32| Ok(ticks_to_duration(*x as i64 + 1)),
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn generate_time_span_throw_iterate() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_generate_after_on(
            0,
            |_: &i32| Ok(true),
            move |_: i32| Err(error.clone()),
            |x: &i32| Ok(*x),
            |x: &i32| Ok(ticks_to_duration(*x as i64 + 1)),
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_next(202, 0), RT::on_error::<i32>(202, ex));
}

#[test]
fn generate_time_span_throw_time_selector() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_generate_after_on(
            0,
            |_: &i32| Ok(true),
            |x: i32| Ok(x + 1),
            |x: &i32| Ok(*x),
            move |_: &i32| -> Result<std::time::Duration, _> { Err(error.clone()) },
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn generate_time_span_dispose() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(
        move || Observable::generate_after_on(0, |_: &i32| true, |x: i32| x + 1, |x: &i32| *x, |x: &i32| ticks_to_duration(*x as i64 + 1), s),
        210,
    );

    assert_messages!(res.messages(), RT::on_next(202, 0), RT::on_next(204, 1), RT::on_next(207, 2));
}

#[test]
fn generate_date_time_offset_finite() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || {
        let now = s.clone();
        Observable::generate_at_on(0, |x: &i32| *x <= 3, |x: i32| x + 1, |x: &i32| *x, move |x: &i32| now.now() + ticks_to_duration(*x as i64 + 1), s)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(202, 0),
        RT::on_next(204, 1),
        RT::on_next(207, 2),
        RT::on_next(211, 3),
        RT::on_completed::<i32>(211)
    );
}

#[test]
fn generate_date_time_offset_throw_condition() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || {
        let now = s.clone();
        Observable::try_generate_at_on(
            0,
            move |_: &i32| Err(error.clone()),
            |x: i32| Ok(x + 1),
            |x: &i32| Ok(*x),
            move |x: &i32| Ok(now.now() + ticks_to_duration(*x as i64 + 1)),
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn generate_date_time_offset_throw_result_selector() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || {
        let now = s.clone();
        Observable::try_generate_at_on(
            0,
            |_: &i32| Ok(true),
            |x: i32| Ok(x + 1),
            move |_: &i32| -> Result<i32, _> { Err(error.clone()) },
            move |x: &i32| Ok(now.now() + ticks_to_duration(*x as i64 + 1)),
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn generate_date_time_offset_throw_iterate() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || {
        let now = s.clone();
        Observable::try_generate_at_on(
            0,
            |_: &i32| Ok(true),
            move |_: i32| Err(error.clone()),
            |x: &i32| Ok(*x),
            move |x: &i32| Ok(now.now() + ticks_to_duration(*x as i64 + 1)),
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_next(202, 0), RT::on_error::<i32>(202, ex));
}

#[test]
fn generate_date_time_offset_throw_time_selector() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let (s, error) = (scheduler.scheduler(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_generate_at_on(
            0,
            |_: &i32| Ok(true),
            |x: i32| Ok(x + 1),
            |x: &i32| Ok(*x),
            move |_: &i32| -> Result<std::time::SystemTime, _> { Err(error.clone()) },
            s,
        )
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex));
}

#[test]
fn generate_date_time_offset_dispose() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(
        move || {
            let now = s.clone();
            Observable::generate_at_on(0, |_: &i32| true, |x: i32| x + 1, |x: &i32| *x, move |x: &i32| now.now() + ticks_to_duration(*x as i64 + 1), s)
        },
        210,
    );

    assert_messages!(res.messages(), RT::on_next(202, 0), RT::on_next(204, 1), RT::on_next(207, 2));
}

// Not ported: Generate_ArgumentChecking - argument validation
// Not ported: Generate_DefaultScheduler_ArgumentChecking - argument validation
// Not ported: Generate_DefaultScheduler - real threads (compares with the sequence on DefaultScheduler.Instance, through the blocking ToEnumerable of AssertEqual)
// Not ported: Generate_LongRunning1 - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: Generate_LongRunning2 - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: Generate_LongRunning_Throw - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: Generate_TimeSpan_ArgumentChecking - argument validation
// Not ported: Generate_TimeSpan_DefaultScheduler_ArgumentChecking - argument validation
// Not ported: Generate_TimeSpan_DefaultScheduler - real threads (compares with the sequence on DefaultScheduler.Instance, through the blocking ToEnumerable of AssertEqual)
// Not ported: Generate_DateTimeOffset_ArgumentChecking - argument validation
// Not ported: Generate_DateTimeOffset_DefaultScheduler_ArgumentChecking - argument validation
// Not ported: Generate_DateTimeOffset_DefaultScheduler - real threads (compares with the sequence on DefaultScheduler.Instance, through the blocking ToEnumerable of AssertEqual)
// Not ported: Generate_TimeSpan_DisposeLater - real threads (Thread.Sleep on the default scheduler)
// Not ported: Generate_DateTimeOffset_DisposeLater - real threads (Thread.Sleep on the default scheduler)
