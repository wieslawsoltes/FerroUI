//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ScanTest.cs` of Rx.NET.
//! Adapted: `Scan(seed, accumulator)` is `scan_with`, `Scan(accumulator)` is `scan`; an
//! accumulator that throws is the `try_` form returning `Err(ex)`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn scan_seed_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let seed = 42;
    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan_with(seed, |acc, x| acc + x));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn scan_seed_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let seed = 42;
    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan_with(seed, |acc, x| acc + x));

    assert_messages!(res.messages(), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn scan_seed_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let seed = 42;
    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan_with(seed, |acc, x| acc + x));

    assert_messages!(res.messages(), RT::on_next(220, seed + 2), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn scan_seed_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");
    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, ex.clone())]);

    let seed = 42;
    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan_with(seed, |acc, x| acc + x));

    assert_messages!(res.messages(), RT::on_error(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn scan_seed_some_data() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let seed = 1;
    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan_with(seed, |acc, x| acc + x));

    assert_messages!(res.messages(),
        RT::on_next(210, seed + 2),
        RT::on_next(220, seed + 2 + 3),
        RT::on_next(230, seed + 2 + 3 + 4),
        RT::on_next(240, seed + 2 + 3 + 4 + 5),
        RT::on_completed(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn scan_seed_accumulator_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let ex = new_error("ex");
    let seed = 1;
    let source = xs.clone();
    let res = {
        let ex = ex.clone();
        scheduler.start(move || {
            source.as_observable().try_scan_with(seed, move |acc, x| {
                if x == 4 {
                    return Err(ex.clone());
                }
                Ok(acc + x)
            })
        })
    };

    assert_messages!(res.messages(), RT::on_next(210, seed + 2), RT::on_next(220, seed + 2 + 3), RT::on_error(230, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn scan_no_seed_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan(|acc, x| acc + x));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn scan_no_seed_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan(|acc, x| acc + x));

    assert_messages!(res.messages(), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn scan_no_seed_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan(|acc, x| acc + x));

    assert_messages!(res.messages(), RT::on_next(220, 2), RT::on_completed(250));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn scan_no_seed_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");
    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, ex.clone())]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan(|acc, x| acc + x));

    assert_messages!(res.messages(), RT::on_error(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn scan_no_seed_some_data() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().scan(|acc, x| acc + x));

    assert_messages!(res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 2 + 3),
        RT::on_next(230, 2 + 3 + 4),
        RT::on_next(240, 2 + 3 + 4 + 5),
        RT::on_completed(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn scan_no_seed_accumulator_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let ex = new_error("ex");
    let source = xs.clone();
    let res = {
        let ex = ex.clone();
        scheduler.start(move || {
            source.as_observable().try_scan(move |acc, x| {
                if x == 4 {
                    return Err(ex.clone());
                }
                Ok(acc + x)
            })
        })
    };

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 2 + 3), RT::on_error(230, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 230));
}

// Not ported: Scan_ArgumentChecking - argument validation
