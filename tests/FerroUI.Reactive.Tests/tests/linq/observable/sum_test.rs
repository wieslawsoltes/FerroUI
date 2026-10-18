//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SumTest.cs` of Rx.NET.
//! Adapted: `int`, `long`, `float`, `double` and their nullable forms are `i32`, `i64`, `f32`, `f64`
//! and `Option` of each (`null` is `None`); `decimal` has no type in the port; the overloads with a
//! selector are `sum_of`; `OverflowException` is `SequenceError::Overflow`.

use ferroui_reactive::linq::SequenceError;
use ferroui_reactive::{ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

/// `e is OverflowException`.
fn is_overflow(error: &ObservableError) -> bool {
    error.downcast_ref::<SequenceError>() == Some(&SequenceError::Overflow)
}

#[test]
fn sum_int32_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_int32_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_int32_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2 + 3 + 4),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_int32_overflow() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, i32::MAX),
        RT::on_next(220, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<i32>(220, is_overflow),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn sum_int32_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn sum_int32_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<i32>(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn sum_int64_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0),
        RT::on_completed::<i64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_int64_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2),
        RT::on_completed::<i64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_int64_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_completed::<i64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2 + 3 + 4),
        RT::on_completed::<i64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_int64_overflow() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, i64::MAX),
        RT::on_next(220, 1),
        RT::on_completed::<i64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<i64>(220, is_overflow),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn sum_int64_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i64>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn sum_int64_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<i64>(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn sum_float_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_completed::<f32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0.0),
        RT::on_completed::<f32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_float_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 2.0),
        RT::on_completed::<f32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0),
        RT::on_completed::<f32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_float_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 2.0),
        RT::on_next(220, 3.0),
        RT::on_next(230, 4.0),
        RT::on_completed::<f32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0 + 3.0 + 4.0),
        RT::on_completed::<f32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_float_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_error::<f32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<f32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn sum_float_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<f32>(150, 1.0),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn sum_double_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_completed::<f64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_double_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 2.0),
        RT::on_completed::<f64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_double_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 2.0),
        RT::on_next(220, 3.0),
        RT::on_next(230, 4.0),
        RT::on_completed::<f64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0 + 3.0 + 4.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_double_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_error::<f64>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<f64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn sum_double_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<f64>(150, 1.0),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn sum_nullable_int32_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(0)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_int32_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_int32_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_next(220, None),
        RT::on_next(230, Some(4)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2 + 4)),
        RT::on_completed::<Option<i32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_int32_overflow() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(i32::MAX)),
        RT::on_next(220, Some(1)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<Option<i32>>(220, is_overflow),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn sum_nullable_int32_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_error::<Option<i32>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<i32>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn sum_nullable_int32_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<i32>>(150, Some(1)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn sum_nullable_int64_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(0)),
        RT::on_completed::<Option<i64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_int64_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2)),
        RT::on_completed::<Option<i64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_int64_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_next(220, None),
        RT::on_next(230, Some(4)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2 + 4)),
        RT::on_completed::<Option<i64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_int64_overflow() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(i64::MAX)),
        RT::on_next(220, Some(1)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<Option<i64>>(220, is_overflow),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn sum_nullable_int64_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_error::<Option<i64>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<i64>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn sum_nullable_int64_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<i64>>(150, Some(1)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn sum_nullable_float_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(0.0)),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_float_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(2.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0)),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_float_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(2.0)),
        RT::on_next(220, None),
        RT::on_next(230, Some(4.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0 + 4.0)),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_float_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_error::<Option<f32>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<f32>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn sum_nullable_float_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<f32>>(150, Some(1.0)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn sum_nullable_double_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(0.0)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_double_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_double_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(2.0)),
        RT::on_next(220, None),
        RT::on_next(230, Some(4.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0 + 4.0)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn sum_nullable_double_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_error::<Option<f64>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<f64>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn sum_nullable_double_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<f64>>(150, Some(1.0)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn sum_selector_regular_int32() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum_of(|x| x.len() as i32)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 6),
        RT::on_completed::<i32>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn sum_selector_regular_int64() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum_of(|x| x.len() as i64)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 6),
        RT::on_completed::<i64>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn sum_selector_regular_single() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum_of(|x| x.len() as f32)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 6.0),
        RT::on_completed::<f32>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn sum_selector_regular_double() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum_of(|x| x.len() as f64)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 6.0),
        RT::on_completed::<f64>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn sum_selector_regular_int32_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum_of(|x| if x == "fo" { None } else { Some(x.len() as i32) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(4)),
        RT::on_completed::<Option<i32>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn sum_selector_regular_int64_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum_of(|x| if x == "fo" { None } else { Some(x.len() as i64) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(4)),
        RT::on_completed::<Option<i64>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn sum_selector_regular_single_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum_of(|x| if x == "fo" { None } else { Some(x.len() as f32) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(4.0)),
        RT::on_completed::<Option<f32>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn sum_selector_regular_double_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "fo"),
        RT::on_next(220, "b"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sum_of(|x| if x == "fo" { None } else { Some(x.len() as f64) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(4.0)),
        RT::on_completed::<Option<f64>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

// Not ported: Sum_ArgumentChecking - argument validation
// Not ported: Sum_Decimal_Empty - no decimal type in ferroui-reactive
// Not ported: Sum_Decimal_Return - no decimal type in ferroui-reactive
// Not ported: Sum_Decimal_Some - no decimal type in ferroui-reactive
// Not ported: Sum_Decimal_Throw - no decimal type in ferroui-reactive
// Not ported: Sum_Decimal_Never - no decimal type in ferroui-reactive
// Not ported: Sum_Nullable_Decimal_Empty - no decimal type in ferroui-reactive
// Not ported: Sum_Nullable_Decimal_Return - no decimal type in ferroui-reactive
// Not ported: Sum_Nullable_Decimal_Some - no decimal type in ferroui-reactive
// Not ported: Sum_Nullable_Decimal_Throw - no decimal type in ferroui-reactive
// Not ported: Sum_Nullable_Decimal_Never - no decimal type in ferroui-reactive
// Not ported: Sum_Selector_Regular_Decimal - no decimal type in ferroui-reactive
// Not ported: Sum_Selector_Regular_Decimal_Nullable - no decimal type in ferroui-reactive
