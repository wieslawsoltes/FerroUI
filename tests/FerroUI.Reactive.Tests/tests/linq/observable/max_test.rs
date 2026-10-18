//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/MaxTest.cs` of Rx.NET.
//! Adapted: `int`, `long`, `float`, `double` and their nullable forms are `i32`, `i64`, `f32`, `f64`
//! and `Option` of each (`null` is `None`); `decimal` has no type in the port; `DateTime` is
//! `SystemTime`; a `string` is a `&str`, and an `Option<&str>` for the tests of the nullable
//! reference type with the default comparer (a `null` is `None`); the overloads with a comparer
//! are `max_with` (`try_max_with` for a comparer that throws), with a selector `max_of` and
//! `max_of_with`; `ReverseComparer` is a closure `|x, y| y.cmp(x)`; `InvalidOperationException` is
//! `SequenceError::NoElements`.

use ferroui_reactive::linq::SequenceError;
use ferroui_reactive::{ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::time::{Duration, SystemTime};

/// `e is InvalidOperationException`.
fn is_invalid_operation(error: &ObservableError) -> bool {
    error.downcast_ref::<SequenceError>().is_some_and(SequenceError::is_invalid_operation)
}

/// `new DateTime(year, month, day)`: midnight of the date (UTC).
fn date(year: i64, month: i64, day: i64) -> SystemTime {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    SystemTime::UNIX_EPOCH + Duration::from_secs((days * 86_400) as u64)
}

#[test]
fn max_int32_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<i32>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_int32_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_int32_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 3),
        RT::on_next(220, 4),
        RT::on_next(230, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 4),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_int32_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_int32_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<i32>(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_int64_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<i64>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_int64_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2),
        RT::on_completed::<i64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_int64_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 3),
        RT::on_next(220, 4),
        RT::on_next(230, 2),
        RT::on_completed::<i64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 4),
        RT::on_completed::<i64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_int64_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i64>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_int64_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<i64>(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_float_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_completed::<f32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<f32>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_float_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 2.0),
        RT::on_completed::<f32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0),
        RT::on_completed::<f32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_float_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 3.0),
        RT::on_next(220, 4.0),
        RT::on_next(230, 2.0),
        RT::on_completed::<f32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 4.0),
        RT::on_completed::<f32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_float_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_error::<f32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<f32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_float_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<f32>(150, 1.0),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_double_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_completed::<f64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<f64>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_double_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 2.0),
        RT::on_completed::<f64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_double_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 3.0),
        RT::on_next(220, 4.0),
        RT::on_next(230, 2.0),
        RT::on_completed::<f64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 4.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_double_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_error::<f64>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<f64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_double_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<f64>(150, 1.0),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_nullable_int32_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, None),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_int32_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_int32_some1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, None),
        RT::on_next(220, Some(4)),
        RT::on_next(230, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(4)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_int32_some2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, None),
        RT::on_next(220, Some(2)),
        RT::on_next(230, Some(4)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(4)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_general_nullable_max_test_lhs_null() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, None),
        RT::on_next(220, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_general_nullable_max_test_rhs_null() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_next(220, None),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_general_nullable_max_test_less() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_next(220, Some(3)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(3)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_general_nullable_max_test_greater() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(3)),
        RT::on_next(220, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(3)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_int32_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_error::<Option<i32>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<i32>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_nullable_int32_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<i32>>(150, Some(1)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_nullable_int64_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, None),
        RT::on_completed::<Option<i64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_int64_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2)),
        RT::on_completed::<Option<i64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_int64_some1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, None),
        RT::on_next(220, Some(4)),
        RT::on_next(230, Some(2)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(4)),
        RT::on_completed::<Option<i64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_int64_some2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, None),
        RT::on_next(220, Some(2)),
        RT::on_next(230, Some(4)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(4)),
        RT::on_completed::<Option<i64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_int64_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_error::<Option<i64>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<i64>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_nullable_int64_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<i64>>(150, Some(1)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_nullable_float_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, None),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_float_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(2.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0)),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_float_some1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, None),
        RT::on_next(220, Some(4.0)),
        RT::on_next(230, Some(2.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(4.0)),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_float_some2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, None),
        RT::on_next(220, Some(2.0)),
        RT::on_next(230, Some(4.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(4.0)),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_float_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_error::<Option<f32>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<f32>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_nullable_float_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<f32>>(150, Some(1.0)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_nullable_double_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, None),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_double_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_double_some1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, None),
        RT::on_next(220, Some(4.0)),
        RT::on_next(230, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(4.0)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_double_some2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, None),
        RT::on_next(220, Some(2.0)),
        RT::on_next(230, Some(4.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(4.0)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_nullable_double_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_error::<Option<f64>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<f64>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_nullable_double_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<f64>>(150, Some(1.0)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_of_t_reference_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some("z")),
        RT::on_completed::<Option<&str>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, None),
        RT::on_completed::<Option<&str>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_value_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, SystemTime::now()),
        RT::on_completed::<SystemTime>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<SystemTime>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_reference_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some("z")),
        RT::on_next(210, Some("a")),
        RT::on_completed::<Option<&str>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some("a")),
        RT::on_completed::<Option<&str>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_value_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, SystemTime::now()),
        RT::on_next(210, date(1983, 2, 11)),
        RT::on_completed::<SystemTime>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, date(1983, 2, 11)),
        RT::on_completed::<SystemTime>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_reference_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some("z")),
        RT::on_next(210, Some("b")),
        RT::on_next(220, Some("c")),
        RT::on_next(230, Some("a")),
        RT::on_completed::<Option<&str>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some("c")),
        RT::on_completed::<Option<&str>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_value_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, SystemTime::now()),
        RT::on_next(210, date(1993, 2, 11)),
        RT::on_next(220, date(2003, 2, 11)),
        RT::on_next(230, date(1983, 2, 11)),
        RT::on_completed::<SystemTime>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, date(2003, 2, 11)),
        RT::on_completed::<SystemTime>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_reference_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some("z")),
        RT::on_error::<Option<&str>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<&str>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_of_t_value_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, SystemTime::now()),
        RT::on_error::<SystemTime>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<SystemTime>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_of_t_reference_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<&str>>(150, Some("z")),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_of_t_value_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<SystemTime>(150, SystemTime::now()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

// A `&str` is never null: the comparer form ends a sequence without a value with
// `SequenceError::NoElements` (the expectation of upstream built with `NO_PERF`; see the
// deviation of `min_with` and `max_with`).
#[test]
fn max_of_t_reference_comparer_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, "z"),
        RT::on_completed::<&str>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<&str>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_value_comparer_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, SystemTime::now()),
        RT::on_completed::<SystemTime>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<SystemTime>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_reference_comparer_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, "z"),
        RT::on_next(210, "a"),
        RT::on_completed::<&str>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, "a"),
        RT::on_completed::<&str>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_value_comparer_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, SystemTime::now()),
        RT::on_next(210, date(1983, 2, 11)),
        RT::on_completed::<SystemTime>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, date(1983, 2, 11)),
        RT::on_completed::<SystemTime>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_reference_comparer_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, "z"),
        RT::on_next(210, "b"),
        RT::on_next(220, "c"),
        RT::on_next(230, "a"),
        RT::on_completed::<&str>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, "a"),
        RT::on_completed::<&str>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_value_comparer_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, SystemTime::now()),
        RT::on_next(210, date(1993, 2, 11)),
        RT::on_next(220, date(2003, 2, 11)),
        RT::on_next(230, date(1983, 2, 11)),
        RT::on_completed::<SystemTime>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, date(1983, 2, 11)),
        RT::on_completed::<SystemTime>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn max_of_t_reference_comparer_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, "z"),
        RT::on_error::<&str>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<&str>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_of_t_value_comparer_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, SystemTime::now()),
        RT::on_error::<SystemTime>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<SystemTime>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn max_of_t_reference_comparer_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<&str>(150, "z"),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_of_t_value_comparer_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<SystemTime>(150, SystemTime::now()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_with(|x, y| y.cmp(x))
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn max_of_t_reference_comparer_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, "z"),
        RT::on_next(210, "b"),
        RT::on_next(220, "c"),
        RT::on_next(230, "a"),
        RT::on_completed::<&str>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_max_with(move |_, _| Err(ex.clone()))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<&str>(220, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn max_of_t_value_comparer_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, SystemTime::now()),
        RT::on_next(210, date(1993, 2, 11)),
        RT::on_next(220, date(2003, 2, 11)),
        RT::on_next(230, date(1983, 2, 11)),
        RT::on_completed::<SystemTime>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_max_with(move |_, _| Err(ex.clone()))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<SystemTime>(220, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn max_selector_regular_int32() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of(|x| x.len() as i32)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 3),
        RT::on_completed::<i32>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn max_selector_regular_int64() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of(|x| x.len() as i64)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 3),
        RT::on_completed::<i64>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn max_selector_regular_single() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of(|x| x.len() as f32)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 3.0),
        RT::on_completed::<f32>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn max_selector_regular_double() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of(|x| x.len() as f64)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 3.0),
        RT::on_completed::<f64>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn max_selector_regular_int32_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of(|x| if x == "fo" { None } else { Some(x.len() as i32) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(3)),
        RT::on_completed::<Option<i32>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn max_selector_regular_int64_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of(|x| if x == "fo" { None } else { Some(x.len() as i64) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(3)),
        RT::on_completed::<Option<i64>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn max_selector_regular_single_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of(|x| if x == "fo" { None } else { Some(x.len() as f32) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(3.0)),
        RT::on_completed::<Option<f32>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn max_selector_regular_double_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of(|x| if x == "fo" { None } else { Some(x.len() as f64) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(3.0)),
        RT::on_completed::<Option<f64>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn max_of_t_selector_regular() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "bar"),
        RT::on_next(220, "qux"),
        RT::on_next(230, "foo"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of(|x| x.chars().rev().collect::<String>())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, "xuq".to_string()),
        RT::on_completed::<String>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn max_of_t_selector_regular_comparer() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "bar"),
        RT::on_next(220, "qux"),
        RT::on_next(230, "foo"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().max_of_with(|x| x.chars().rev().collect::<String>(), |x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, "oof".to_string()),
        RT::on_completed::<String>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

// Not ported: Max_ArgumentChecking - argument validation
// Not ported: Max_Decimal_Empty - no decimal type in ferroui-reactive
// Not ported: Max_Decimal_Return - no decimal type in ferroui-reactive
// Not ported: Max_Decimal_Some - no decimal type in ferroui-reactive
// Not ported: Max_Decimal_Throw - no decimal type in ferroui-reactive
// Not ported: Max_Decimal_Never - no decimal type in ferroui-reactive
// Not ported: Max_Nullable_Decimal_Empty - no decimal type in ferroui-reactive
// Not ported: Max_Nullable_Decimal_Return - no decimal type in ferroui-reactive
// Not ported: Max_Nullable_Decimal_Some1 - no decimal type in ferroui-reactive
// Not ported: Max_Nullable_Decimal_Some2 - no decimal type in ferroui-reactive
// Not ported: Max_Nullable_Decimal_Throw - no decimal type in ferroui-reactive
// Not ported: Max_Nullable_Decimal_Never - no decimal type in ferroui-reactive
// Not ported: Max_Selector_Regular_Decimal - no decimal type in ferroui-reactive
// Not ported: Max_Selector_Regular_Decimal_Nullable - no decimal type in ferroui-reactive
