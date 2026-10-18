//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AggregateTest.cs` of Rx.NET.
//! Adapted: the overloads with a seed are `aggregate_with_seed` and
//! `aggregate_with_seed_and_result`; an accumulator or result selector that throws is the `try_`
//! form returning `Err(ex)`; `InvalidOperationException` is `SequenceError::NoElements`.

use ferroui_reactive::linq::SequenceError;
use ferroui_reactive::{ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

/// `e is InvalidOperationException`.
fn is_invalid_operation(error: &ObservableError) -> bool {
    error.downcast_ref::<SequenceError>().is_some_and(SequenceError::is_invalid_operation)
}

#[test]
fn aggregate_with_seed_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed(42, |acc, x| acc + x)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 42),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn aggregate_with_seed_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 24),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed(42, |acc, x| acc + x)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 42 + 24),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn aggregate_with_seed_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed(42, |acc, x| acc + x)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn aggregate_with_seed_never() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed(42, |acc, x| acc + x)
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn aggregate_with_seed_range() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 0),
        RT::on_next(220, 1),
        RT::on_next(230, 2),
        RT::on_next(240, 3),
        RT::on_next(250, 4),
        RT::on_completed::<i32>(260),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed(42, |acc, x| acc + x)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(260, 42 + (0..5).sum::<i32>()),
        RT::on_completed::<i32>(260),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 260));
}

#[test]
fn aggregate_with_seed_accumulator_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 0),
        RT::on_next(220, 1),
        RT::on_next(230, 2),
        RT::on_next(240, 3),
        RT::on_next(250, 4),
        RT::on_completed::<i32>(260),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_aggregate_with_seed(0, move |acc, x| if x < 3 { Ok(acc + x) } else { Err(ex.clone()) })
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(240, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn aggregate_with_seed_and_result_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed_and_result(42, |acc, x| acc + x, |x| x * 5)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 42 * 5),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn aggregate_with_seed_and_result_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 24),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed_and_result(42, |acc, x| acc + x, |x| x * 5)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, (42 + 24) * 5),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn aggregate_with_seed_and_result_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed_and_result(42, |acc, x| acc + x, |x| x * 5)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn aggregate_with_seed_and_result_never() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed_and_result(42, |acc, x| acc + x, |x| x * 5)
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn aggregate_with_seed_and_result_range() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 0),
        RT::on_next(220, 1),
        RT::on_next(230, 2),
        RT::on_next(240, 3),
        RT::on_next(250, 4),
        RT::on_completed::<i32>(260),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate_with_seed_and_result(42, |acc, x| acc + x, |x| x * 5)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(260, (42 + (0..5).sum::<i32>()) * 5),
        RT::on_completed::<i32>(260),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 260));
}

#[test]
fn aggregate_with_seed_and_result_accumulator_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 0),
        RT::on_next(220, 1),
        RT::on_next(230, 2),
        RT::on_next(240, 3),
        RT::on_next(250, 4),
        RT::on_completed::<i32>(260),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_aggregate_with_seed_and_result(0, move |acc, x| if x < 3 { Ok(acc + x) } else { Err(ex.clone()) }, |x| Ok(x * 5))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(240, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn aggregate_with_seed_and_result_result_selector_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 0),
        RT::on_next(220, 1),
        RT::on_next(230, 2),
        RT::on_next(240, 3),
        RT::on_next(250, 4),
        RT::on_completed::<i32>(260),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_aggregate_with_seed_and_result(0, |acc, x| Ok(acc + x), move |_: i32| Err::<i32, _>(ex.clone()))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(260, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 260));
}

#[test]
fn aggregate_without_seed_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate(|acc, x| acc + x)
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<i32>(250, is_invalid_operation),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn aggregate_without_seed_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 24),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate(|acc, x| acc + x)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 24),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn aggregate_without_seed_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate(|acc, x| acc + x)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn aggregate_without_seed_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate(|acc, x| acc + x)
    });

    assert_messages!(res.messages());
}

#[test]
fn aggregate_without_seed_range() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 0),
        RT::on_next(220, 1),
        RT::on_next(230, 2),
        RT::on_next(240, 3),
        RT::on_next(250, 4),
        RT::on_completed::<i32>(260),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().aggregate(|acc, x| acc + x)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(260, (0..5).sum::<i32>()),
        RT::on_completed::<i32>(260),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 260));
}

#[test]
fn aggregate_without_seed_accumulator_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 0),
        RT::on_next(220, 1),
        RT::on_next(230, 2),
        RT::on_next(240, 3),
        RT::on_next(250, 4),
        RT::on_completed::<i32>(260),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_aggregate(move |acc, x| if x < 3 { Ok(acc + x) } else { Err(ex.clone()) })
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<i32>(240, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 240));
}

// Not ported: Aggregate_ArgumentChecking - argument validation
