//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/LongCountTest.cs` of Rx.NET.
//! Adapted: the count is a `u64`; the overload with a predicate is `long_count_with`; a predicate
//! that throws is `try_long_count_with` returning `Err(ex)`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn long_count_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 1),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_some() {
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
        move || xs.as_observable().long_count()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<u64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn long_count_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn long_count_predicate_empty_true() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count_with(|_| true)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_predicate_empty_false() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count_with(|_| false)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_predicate_return_true() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count_with(|_| true)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 1),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_predicate_return_false() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count_with(|_| false)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_predicate_some_all() {
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
        move || xs.as_observable().long_count_with(|x| *x < 10)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 3),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_predicate_some_none() {
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
        move || xs.as_observable().long_count_with(|x| *x > 10)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 0),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_predicate_some_even() {
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
        move || xs.as_observable().long_count_with(|x| x % 2 == 0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 2),
        RT::on_completed::<u64>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn long_count_predicate_throw_true() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count_with(|_| true)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<u64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn long_count_predicate_throw_false() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count_with(|_| false)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<u64>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn long_count_predicate_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().long_count_with(|_| true)
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn long_count_predicate_predicate_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(240),
    ]);

    let ex = new_error("ex");

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_long_count_with(move |x| if *x == 3 { Err(ex.clone()) } else { Ok(true) })
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<u64>(230, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 230));
}

// Not ported: LongCount_ArgumentChecking - argument validation
// Not ported: LongCount_InjectOverflow - sets the private count of the sink through reflection (and runs on the thread pool)
// Not ported: LongCount_Predicate_InjectOverflow - sets the private count of the sink through reflection (and runs on the thread pool)
