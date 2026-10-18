//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/MinByTest.cs` of Rx.NET.
//! Adapted: a `KeyValuePair<int, string>` is a tuple `(i32, &str)`; the list of the result is a
//! `Vec`, compared with the expected one; the overload with a comparer is `min_by_with`
//! (`ReverseComparer` is a closure `|x, y| y.cmp(x)`); a key selector or comparer that throws is the
//! `try_` form returning `Err(ex)`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn min_by_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_completed::<(i32, &str)>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by(|x| x.0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Vec::<(i32, &str)>::new()),
        RT::on_completed::<Vec<(i32, &str)>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn min_by_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_next(210, (2, "a")),
        RT::on_completed::<(i32, &str)>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by(|x| x.0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, vec![(2, "a")]),
        RT::on_completed::<Vec<(i32, &str)>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn min_by_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_next(210, (3, "b")),
        RT::on_next(220, (2, "c")),
        RT::on_next(230, (4, "a")),
        RT::on_completed::<(i32, &str)>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by(|x| x.0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, vec![(2, "c")]),
        RT::on_completed::<Vec<(i32, &str)>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn min_by_multiple() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_next(210, (3, "b")),
        RT::on_next(215, (2, "d")),
        RT::on_next(220, (3, "c")),
        RT::on_next(225, (2, "y")),
        RT::on_next(230, (4, "a")),
        RT::on_next(235, (4, "r")),
        RT::on_completed::<(i32, &str)>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by(|x| x.0)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, vec![(2, "d"), (2, "y")]),
        RT::on_completed::<Vec<(i32, &str)>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn min_by_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_error::<(i32, &str)>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by(|x| x.0)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Vec<(i32, &str)>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn min_by_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<(i32, &str)>(150, (1, "z")),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by(|x| x.0)
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn min_by_comparer_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_completed::<(i32, &str)>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by_with(|x| x.0, |x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, Vec::<(i32, &str)>::new()),
        RT::on_completed::<Vec<(i32, &str)>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn min_by_comparer_return() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_next(210, (2, "a")),
        RT::on_completed::<(i32, &str)>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by_with(|x| x.0, |x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, vec![(2, "a")]),
        RT::on_completed::<Vec<(i32, &str)>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn min_by_comparer_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_next(210, (3, "b")),
        RT::on_next(220, (20, "c")),
        RT::on_next(230, (4, "a")),
        RT::on_completed::<(i32, &str)>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by_with(|x| x.0, |x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, vec![(20, "c")]),
        RT::on_completed::<Vec<(i32, &str)>>(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn min_by_comparer_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_error::<(i32, &str)>(210, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by_with(|x| x.0, |x, y| y.cmp(x))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Vec<(i32, &str)>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn min_by_comparer_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<(i32, &str)>(150, (1, "z")),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().min_by_with(|x| x.0, |x, y| y.cmp(x))
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn min_by_selector_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_next(210, (3, "b")),
        RT::on_next(220, (2, "c")),
        RT::on_next(230, (4, "a")),
        RT::on_completed::<(i32, &str)>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_min_by(move |_| Err::<i32, _>(ex.clone()))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Vec<(i32, &str)>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn min_by_comparer_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, (1, "z")),
        RT::on_next(210, (3, "b")),
        RT::on_next(220, (2, "c")),
        RT::on_next(230, (4, "a")),
        RT::on_completed::<(i32, &str)>(250),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_min_by_with(|x| Ok(x.0), move |_, _| Err(ex.clone()))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Vec<(i32, &str)>>(220, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

// Not ported: MinBy_ArgumentChecking - argument validation
