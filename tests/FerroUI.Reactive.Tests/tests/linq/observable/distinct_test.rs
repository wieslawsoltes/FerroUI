//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DistinctTest.cs` of Rx.NET.
//! Adapted: an `IEqualityComparer<T>` is a function of equality (`ModComparer` compares the
//! remainders); a key selector that throws is the `try_` form returning the error; `ThrowComparer`,
//! whose hash code throws for one value, is a comparer that fails when that value is the key being
//! added (the port compares a new key with the keys seen before); `null` strings are `None`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

/// `ModComparer`: values are equal when their remainders by `modulus` are.
fn mod_comparer(modulus: i32) -> impl Fn(&i32, &i32) -> bool + 'static {
    move |x, y| x % modulus == y % modulus
}

#[test]
fn distinct_default_comparer_all_distinct() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 4),
        RT::on_next(300, 2),
        RT::on_next(350, 1),
        RT::on_next(380, 3),
        RT::on_next(400, 5),
        RT::on_completed(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(280, 4),
        RT::on_next(300, 2),
        RT::on_next(350, 1),
        RT::on_next(380, 3),
        RT::on_next(400, 5),
        RT::on_completed(420)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn distinct_default_comparer_some_duplicates() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 4),
        RT::on_next(300, 2),
        RT::on_next(350, 2),
        RT::on_next(380, 3),
        RT::on_next(400, 4),
        RT::on_completed(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct()
    });

    assert_messages!(res.messages(), RT::on_next(280, 4), RT::on_next(300, 2), RT::on_next(380, 3), RT::on_completed(420));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn distinct_custom_comparer_all_distinct() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 4),
        RT::on_next(300, 2),
        RT::on_next(350, 1),
        RT::on_next(380, 3),
        RT::on_next(400, 5),
        RT::on_completed(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_with_comparer(mod_comparer(10))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(280, 4),
        RT::on_next(300, 2),
        RT::on_next(350, 1),
        RT::on_next(380, 3),
        RT::on_next(400, 5),
        RT::on_completed(420)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn distinct_custom_comparer_some_duplicates() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 4),
        RT::on_next(300, 2),
        RT::on_next(350, 12),
        RT::on_next(380, 3),
        RT::on_next(400, 24),
        RT::on_completed(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_with_comparer(mod_comparer(10))
    });

    assert_messages!(res.messages(), RT::on_next(280, 4), RT::on_next(300, 2), RT::on_next(380, 3), RT::on_completed(420));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn distinct_key_selector_default_comparer_all_distinct() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 8),
        RT::on_next(300, 4),
        RT::on_next(350, 2),
        RT::on_next(380, 6),
        RT::on_next(400, 10),
        RT::on_completed(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_by_key(|x| x / 2)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(280, 8),
        RT::on_next(300, 4),
        RT::on_next(350, 2),
        RT::on_next(380, 6),
        RT::on_next(400, 10),
        RT::on_completed(420)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn distinct_key_selector_default_comparer_some_duplicates() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 4),
        RT::on_next(300, 2),
        RT::on_next(350, 3),
        RT::on_next(380, 7),
        RT::on_next(400, 5),
        RT::on_completed(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_by_key(|x| x / 2)
    });

    assert_messages!(res.messages(), RT::on_next(280, 4), RT::on_next(300, 2), RT::on_next(380, 7), RT::on_completed(420));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn distinct_key_selector_custom_comparer_all_distinct() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 8),
        RT::on_next(300, 4),
        RT::on_next(350, 2),
        RT::on_next(380, 6),
        RT::on_next(400, 10),
        RT::on_completed(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_with(|x| x / 2, mod_comparer(10))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(280, 8),
        RT::on_next(300, 4),
        RT::on_next(350, 2),
        RT::on_next(380, 6),
        RT::on_next(400, 10),
        RT::on_completed(420)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn distinct_key_selector_custom_comparer_some_duplicates() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 8),
        RT::on_next(300, 4),
        RT::on_next(350, 2),
        RT::on_next(380, 6),
        RT::on_next(400, 10),
        RT::on_completed(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct_with(|x| x / 2, mod_comparer(3))
    });

    assert_messages!(res.messages(), RT::on_next(280, 8), RT::on_next(300, 4), RT::on_next(380, 6), RT::on_completed(420));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn distinct_key_selector_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 3),
        RT::on_next(300, 2),
        RT::on_next(350, 1),
        RT::on_next(380, 0),
        RT::on_next(400, 4),
        RT::on_completed(420),
    ]);

    let ex = new_error("ex");

    let res = scheduler.start({
        let (xs, ex) = (xs.clone(), ex.clone());
        move || xs.as_observable().try_distinct_by_key(move |x| if *x == 0 { Err(ex.clone()) } else { Ok(x / 2) })
    });

    assert_messages!(res.messages(), RT::on_next(280, 3), RT::on_next(350, 1), RT::on_error(380, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 380));
}

#[test]
fn distinct_custom_comparer_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(280, 1),
        RT::on_next(300, 2),
        RT::on_next(350, 3),
        RT::on_next(380, 4),
        RT::on_next(400, 5),
        RT::on_completed(420),
    ]);

    let ex = new_error("ex");

    // `ThrowComparer(4, ex)`: the comparison of the key 4 fails.
    let res = scheduler.start({
        let (xs, ex) = (xs.clone(), ex.clone());
        move || xs.as_observable().try_distinct_with_comparer(move |x, y| if *y == 4 { Err(ex.clone()) } else { Ok(x == y) })
    });

    assert_messages!(res.messages(), RT::on_next(280, 1), RT::on_next(300, 2), RT::on_next(350, 3), RT::on_error(380, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 380));
}

#[test]
fn distinct_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(280, 1), RT::on_next(300, 2), RT::on_next(350, 3), RT::on_error(380, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct()
    });

    assert_messages!(res.messages(), RT::on_next(280, 1), RT::on_next(300, 2), RT::on_next(350, 3), RT::on_error(380, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 380));
}

#[test]
fn distinct_null() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(220, Some("bar")),
        RT::on_next(240, None::<&'static str>),
        RT::on_next(260, Some("bar")),
        RT::on_next(280, Some("foo")),
        RT::on_next(300, None),
        RT::on_completed(400),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().distinct()
    });

    assert_messages!(res.messages(), RT::on_next(220, Some("bar")), RT::on_next(240, None), RT::on_next(280, Some("foo")), RT::on_completed(400));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

// Not ported: Distinct_ArgumentChecking - argument validation
