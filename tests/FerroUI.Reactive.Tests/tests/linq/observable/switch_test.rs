//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SwitchTest.cs` of Rx.NET.
//! Adapted: the inner sequences of the sequence of sequences are `Rc<dyn IObservable<i32>>` handles
//! of the test observables. `Switch` of a sequence of tasks is `switch` (a `Task` is an
//! `ObservableSource`); `Task.Factory.StartNew` is `tasks::run` and the blocking `Single()` is
//! `block_on` of `single_async().to_task()`.

use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::tasks::{block_on, run};
use std::rc::Rc;

type Inner = Rc<dyn IObservable<i32>>;

#[test]
fn switch_data() {
    let scheduler = TestScheduler::new();

    let ys1 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 101),
        RT::on_next(20, 102),
        RT::on_next(110, 103),
        RT::on_next(120, 104),
        RT::on_next(210, 105),
        RT::on_next(220, 106),
        RT::on_completed(230),
    ]);

    let ys2 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 201),
        RT::on_next(20, 202),
        RT::on_next(30, 203),
        RT::on_next(40, 204),
        RT::on_completed(50),
    ]);

    let ys3 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 301),
        RT::on_next(20, 302),
        RT::on_next(30, 303),
        RT::on_next(40, 304),
        RT::on_completed(150),
    ]);

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(300, ys1.as_observable()),
        RT::on_next::<Inner>(400, ys2.as_observable()),
        RT::on_next::<Inner>(500, ys3.as_observable()),
        RT::on_completed::<Inner>(600),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.switch());

    assert_messages!(
        res.messages(),
        RT::on_next(310, 101),
        RT::on_next(320, 102),
        RT::on_next(410, 201),
        RT::on_next(420, 202),
        RT::on_next(430, 203),
        RT::on_next(440, 204),
        RT::on_next(510, 301),
        RT::on_next(520, 302),
        RT::on_next(530, 303),
        RT::on_next(540, 304),
        RT::on_completed(650)
    );

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(300, 400));

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(400, 450));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(500, 650));
}

#[test]
fn switch_inner_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let ys1 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 101),
        RT::on_next(20, 102),
        RT::on_next(110, 103),
        RT::on_next(120, 104),
        RT::on_next(210, 105),
        RT::on_next(220, 106),
        RT::on_completed(230),
    ]);

    let ys2 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 201),
        RT::on_next(20, 202),
        RT::on_next(30, 203),
        RT::on_next(40, 204),
        RT::on_error(50, ex.clone()),
    ]);

    let ys3 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 301),
        RT::on_next(20, 302),
        RT::on_next(30, 303),
        RT::on_next(40, 304),
        RT::on_completed(150),
    ]);

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(300, ys1.as_observable()),
        RT::on_next::<Inner>(400, ys2.as_observable()),
        RT::on_next::<Inner>(500, ys3.as_observable()),
        RT::on_completed::<Inner>(600),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.switch());

    assert_messages!(
        res.messages(),
        RT::on_next(310, 101),
        RT::on_next(320, 102),
        RT::on_next(410, 201),
        RT::on_next(420, 202),
        RT::on_next(430, 203),
        RT::on_next(440, 204),
        RT::on_error(450, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(300, 400));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(400, 450));

    assert_subscriptions!(ys3.subscriptions());
}

#[test]
fn switch_outer_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let ys1 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 101),
        RT::on_next(20, 102),
        RT::on_next(110, 103),
        RT::on_next(120, 104),
        RT::on_next(210, 105),
        RT::on_next(220, 106),
        RT::on_completed(230),
    ]);

    let ys2 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 201),
        RT::on_next(20, 202),
        RT::on_next(30, 203),
        RT::on_next(40, 204),
        RT::on_completed(50),
    ]);

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(300, ys1.as_observable()),
        RT::on_next::<Inner>(400, ys2.as_observable()),
        RT::on_error::<Inner>(500, ex.clone()),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.switch());

    assert_messages!(
        res.messages(),
        RT::on_next(310, 101),
        RT::on_next(320, 102),
        RT::on_next(410, 201),
        RT::on_next(420, 202),
        RT::on_next(430, 203),
        RT::on_next(440, 204),
        RT::on_error(500, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 500));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(300, 400));

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(400, 450));
}

#[test]
fn switch_no_inner() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<Inner>(500)]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.switch());

    assert_messages!(res.messages(), RT::on_completed::<i32>(500));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn switch_inner_completes() {
    let scheduler = TestScheduler::new();

    let ys1 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 101),
        RT::on_next(20, 102),
        RT::on_next(110, 103),
        RT::on_next(120, 104),
        RT::on_next(210, 105),
        RT::on_next(220, 106),
        RT::on_completed(230),
    ]);

    let xs = scheduler.create_hot_observable(vec![RT::on_next::<Inner>(300, ys1.as_observable()), RT::on_completed::<Inner>(540)]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.switch());

    assert_messages!(
        res.messages(),
        RT::on_next(310, 101),
        RT::on_next(320, 102),
        RT::on_next(410, 103),
        RT::on_next(420, 104),
        RT::on_next(510, 105),
        RT::on_next(520, 106),
        RT::on_completed(540)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 540));

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(300, 530));
}

#[test]
fn switch_task() {
    let tss = Observable::to_observable(vec![run(|| Ok(1)), run(|| Ok(2)), run(|| Ok(3))]).switch::<i32>();

    let res = block_on(tss.to_array().single_async().to_task()).unwrap();

    assert!(res.iter().zip(res.iter().skip(1)).map(|(l, r)| r > l).all(|b| b));
}

// Not ported: Switch_ArgumentChecking - argument validation
