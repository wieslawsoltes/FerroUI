//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AverageTest.cs` of Rx.NET.
//! Adapted: `int`, `long`, `float`, `double` and their nullable forms are `i32`, `i64`, `f32`, `f64`
//! and `Option` of each (`null` is `None`); `decimal` has no type in the port; the overloads with a
//! selector are `average_of`; `InvalidOperationException` is `SequenceError::NoElements`.

use ferroui_reactive::linq::SequenceError;
use ferroui_reactive::{ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

/// `e is InvalidOperationException`.
fn is_invalid_operation(error: &ObservableError) -> bool {
    error.downcast_ref::<SequenceError>().is_some_and(SequenceError::is_invalid_operation)
}

#[test]
fn average_int32_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<f64>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_int32_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_int32_some() {
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
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_int32_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<f64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn average_int32_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<i32>(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn average_int64_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<f64>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_int64_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_int64_some() {
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
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_int64_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i64>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<f64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn average_int64_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<i64>(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn average_double_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_completed::<f64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<f64>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_double_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 2.0),
        RT::on_completed::<f64>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_double_some() {
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
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3.0),
        RT::on_completed::<f64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_double_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_error::<f64>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<f64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn average_double_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<f64>(150, 1.0),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn average_float_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_completed::<f32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<f32>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_float_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_next(210, 2.0),
        RT::on_completed::<f32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2.0),
        RT::on_completed::<f32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_float_some() {
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
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3.0),
        RT::on_completed::<f32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_float_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1.0),
        RT::on_error::<f32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<f32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn average_float_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<f32>(150, 1.0),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn average_nullable_int32_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, None),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_int32_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_int32_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(3)),
        RT::on_next(220, None),
        RT::on_next(230, Some(2)),
        RT::on_completed::<Option<i32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.5)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_int32_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_error::<Option<i32>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<f64>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn average_nullable_int32_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<i32>>(150, Some(1)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn average_nullable_int64_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, None),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_int64_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(2)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_int64_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_next(210, Some(3)),
        RT::on_next(220, None),
        RT::on_next(230, Some(2)),
        RT::on_completed::<Option<i64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.5)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_int64_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1)),
        RT::on_error::<Option<i64>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<f64>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn average_nullable_int64_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<i64>>(150, Some(1)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn average_nullable_double_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, None),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_double_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_double_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(3.0)),
        RT::on_next(220, None),
        RT::on_next(230, Some(2.0)),
        RT::on_completed::<Option<f64>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.5)),
        RT::on_completed::<Option<f64>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_double_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_error::<Option<f64>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<f64>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn average_nullable_double_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<f64>>(150, Some(1.0)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn average_nullable_float_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, None),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_float_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(2.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.0)),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_float_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_next(210, Some(3.0)),
        RT::on_next(220, None),
        RT::on_next(230, Some(2.0)),
        RT::on_completed::<Option<f32>>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Some(2.5)),
        RT::on_completed::<Option<f32>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn average_nullable_float_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, Some(1.0)),
        RT::on_error::<Option<f32>>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Option<f32>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn average_nullable_float_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Option<f32>>(150, Some(1.0)),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn average_selector_regular_int32() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "b"),
        RT::on_next(220, "fo"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average_of(|x| x.len() as i32)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 2.0),
        RT::on_completed::<f64>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn average_selector_regular_int64() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "b"),
        RT::on_next(220, "fo"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average_of(|x| x.len() as i64)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 2.0),
        RT::on_completed::<f64>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn average_selector_regular_single() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "b"),
        RT::on_next(220, "fo"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average_of(|x| x.len() as f32)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 2.0),
        RT::on_completed::<f32>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn average_selector_regular_double() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "b"),
        RT::on_next(220, "fo"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average_of(|x| x.len() as f64)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 2.0),
        RT::on_completed::<f64>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn average_selector_regular_int32_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "b"),
        RT::on_next(220, "fo"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average_of(|x| if x == "fo" { None } else { Some(x.len() as i32) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(2.0)),
        RT::on_completed::<Option<f64>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn average_selector_regular_int64_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "b"),
        RT::on_next(220, "fo"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average_of(|x| if x == "fo" { None } else { Some(x.len() as i64) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(2.0)),
        RT::on_completed::<Option<f64>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn average_selector_regular_single_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "b"),
        RT::on_next(220, "fo"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average_of(|x| if x == "fo" { None } else { Some(x.len() as f32) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(2.0)),
        RT::on_completed::<Option<f32>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn average_selector_regular_double_nullable() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, "b"),
        RT::on_next(220, "fo"),
        RT::on_next(230, "qux"),
        RT::on_completed::<&str>(240),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().average_of(|x| if x == "fo" { None } else { Some(x.len() as f64) })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, Some(2.0)),
        RT::on_completed::<Option<f64>>(240),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

// Not ported: Average_ArgumentChecking - argument validation
// Not ported: Average_Decimal_Empty - no decimal type in ferroui-reactive
// Not ported: Average_Decimal_Return - no decimal type in ferroui-reactive
// Not ported: Average_Decimal_Some - no decimal type in ferroui-reactive
// Not ported: Average_Decimal_Throw - no decimal type in ferroui-reactive
// Not ported: Average_Decimal_Never - no decimal type in ferroui-reactive
// Not ported: Average_Nullable_Decimal_Empty - no decimal type in ferroui-reactive
// Not ported: Average_Nullable_Decimal_Return - no decimal type in ferroui-reactive
// Not ported: Average_Nullable_Decimal_Some - no decimal type in ferroui-reactive
// Not ported: Average_Nullable_Decimal_Throw - no decimal type in ferroui-reactive
// Not ported: Average_Nullable_Decimal_Never - no decimal type in ferroui-reactive
// Not ported: Average_InjectOverflow_Int32 - sets the private count of the sink through reflection (and runs on the thread pool)
// Not ported: Average_InjectOverflow_Int64 - sets the private count of the sink through reflection (and runs on the thread pool)
// Not ported: Average_InjectOverflow_Double - sets the private count of the sink through reflection (and runs on the thread pool)
// Not ported: Average_InjectOverflow_Single - sets the private count of the sink through reflection (and runs on the thread pool)
// Not ported: Average_InjectOverflow_Decimal - no decimal type in ferroui-reactive
// Not ported: Average_InjectOverflow_Int32_Nullable - sets the private count of the sink through reflection (and runs on the thread pool)
// Not ported: Average_InjectOverflow_Int64_Nullable - sets the private count of the sink through reflection (and runs on the thread pool)
// Not ported: Average_InjectOverflow_Double_Nullable - sets the private count of the sink through reflection (and runs on the thread pool)
// Not ported: Average_InjectOverflow_Single_Nullable - sets the private count of the sink through reflection (and runs on the thread pool)
// Not ported: Average_InjectOverflow_Decimal_Nullable - no decimal type in ferroui-reactive
// Not ported: Average_Selector_Regular_Decimal - no decimal type in ferroui-reactive
// Not ported: Average_Selector_Regular_Decimal_Nullable - no decimal type in ferroui-reactive
