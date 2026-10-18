//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ToListTest.cs` of Rx.NET.
//! Adapted: the list is a `Vec<T>`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn to_list_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(220, 2),
        RT::on_next(330, 3),
        RT::on_next(440, 4),
        RT::on_next(550, 5),
        RT::on_completed::<i32>(660),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().to_list()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(660, vec![2, 3, 4, 5]),
        RT::on_completed::<Vec<i32>>(660),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 660));
}

#[test]
fn to_list_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(220, 2),
        RT::on_next(330, 3),
        RT::on_next(440, 4),
        RT::on_next(550, 5),
        RT::on_error::<i32>(660, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().to_list()
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Vec<i32>>(660, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 660));
}

#[test]
fn to_list_disposed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(220, 2),
        RT::on_next(330, 3),
        RT::on_next(440, 4),
        RT::on_next(550, 5),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().to_list()
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

// Not ported: ToList_ArgumentChecking - argument validation
