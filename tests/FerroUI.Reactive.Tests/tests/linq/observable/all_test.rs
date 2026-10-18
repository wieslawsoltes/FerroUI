//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AllTest.cs` of Rx.NET.
//! Adapted: a predicate that throws is `try_all` returning `Err(ex)`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn all_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().all(|x| *x > 0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, true),
        RT::on_completed::<bool>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn all_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().all(|x| *x > 0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, true),
        RT::on_completed::<bool>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn all_return_not_match() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, -2),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().all(|x| *x > 0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, false),
        RT::on_completed::<bool>(210),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn all_some_none_match() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, -2),
        RT::on_next(220, -3),
        RT::on_next(230, -4),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().all(|x| *x > 0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, false),
        RT::on_completed::<bool>(210),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn all_some_match() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, -2),
        RT::on_next(220, 3),
        RT::on_next(230, -4),
        RT::on_completed::<i32>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().all(|x| *x > 0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, false),
        RT::on_completed::<bool>(210),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn all_some_all_match() {
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
        move || xs.as_observable().all(|x| *x > 0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, true),
        RT::on_completed::<bool>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn all_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().all(|x| *x > 0)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn all_never() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().all(|x| *x > 0)
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn all_predicate_throws() {
    let ex = new_error("ex");

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
        let ex = ex.clone();
        move || xs.as_observable().try_all(move |x| if *x < 4 { Ok(true) } else { Err(ex.clone()) })
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(230, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 230));
}

// Not ported: All_ArgumentChecking - argument validation
