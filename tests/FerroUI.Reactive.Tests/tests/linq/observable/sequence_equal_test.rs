//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SequenceEqualTest.cs` of Rx.NET.
//! Adapted: the comparer classes are closures (`sequence_equal_with`, `sequence_equal_iter_with`);
//! comparers that throw are the `try_` forms returning `Err(ex)`; the enumerable is a `Vec`
//! (`sequence_equal_iter`), and the enumerables whose iterator or `GetEnumerator` throws are
//! `try_sequence_equal_iter_with` with an iterator of `Result` items or a factory returning
//! `Err(ex)`.

use ferroui_reactive::{ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

/// The comparer of `OddEvenComparer`: equal when the parity is.
fn odd_even(x: &i32, y: &i32) -> bool {
    x % 2 == y % 2
}

/// The items of the iterator `Throw(ex)`: 3, then `MoveNext` throws `ex`.
fn throw_after_3(ex: &ObservableError) -> Vec<Result<i32, ObservableError>> {
    vec![Ok(3), Err(ex.clone())]
}

#[test]
fn sequence_equal_observable_equal() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(280, 4),
        RT::on_next(300, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed::<i32>(720),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || xs.as_observable().sequence_equal(ys.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(720, true),
        RT::on_completed::<bool>(720),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 720));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 720));
}

#[test]
fn sequence_equal_observable_equal_sym() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(280, 4),
        RT::on_next(300, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed::<i32>(720),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || ys.as_observable().sequence_equal(xs.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(720, true),
        RT::on_completed::<bool>(720),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 720));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 720));
}

#[test]
fn sequence_equal_observable_not_equal_left() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 0),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(280, 4),
        RT::on_next(300, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed::<i32>(720),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || xs.as_observable().sequence_equal(ys.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, false),
        RT::on_completed::<bool>(310),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn sequence_equal_observable_not_equal_left_sym() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 0),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(280, 4),
        RT::on_next(300, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed::<i32>(720),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || ys.as_observable().sequence_equal(xs.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, false),
        RT::on_completed::<bool>(310),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn sequence_equal_observable_not_equal_right() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(280, 4),
        RT::on_next(300, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_next(350, 8),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || xs.as_observable().sequence_equal(ys.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(510, false),
        RT::on_completed::<bool>(510),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 510));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 510));
}

#[test]
fn sequence_equal_observable_not_equal_right_sym() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(280, 4),
        RT::on_next(300, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_next(350, 8),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || ys.as_observable().sequence_equal(xs.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(510, false),
        RT::on_completed::<bool>(510),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 510));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 510));
}

#[test]
fn sequence_equal_observable_not_equal_2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_next(490, 8),
        RT::on_next(520, 9),
        RT::on_next(580, 10),
        RT::on_next(600, 11),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(280, 4),
        RT::on_next(300, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_next(350, 9),
        RT::on_next(400, 9),
        RT::on_next(410, 10),
        RT::on_next(490, 11),
        RT::on_next(550, 12),
        RT::on_next(560, 13),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || xs.as_observable().sequence_equal(ys.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(490, false),
        RT::on_completed::<bool>(490),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 490));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 490));
}

#[test]
fn sequence_equal_observable_not_equal_2_sym() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_next(490, 8),
        RT::on_next(520, 9),
        RT::on_next(580, 10),
        RT::on_next(600, 11),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(280, 4),
        RT::on_next(300, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_next(350, 9),
        RT::on_next(400, 9),
        RT::on_next(410, 10),
        RT::on_next(490, 11),
        RT::on_next(550, 12),
        RT::on_next(560, 13),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || ys.as_observable().sequence_equal(xs.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(490, false),
        RT::on_completed::<bool>(490),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 490));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 490));
}

#[test]
fn sequence_equal_observable_not_equal_3() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_completed::<i32>(330),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(400, 4),
        RT::on_completed::<i32>(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || xs.as_observable().sequence_equal(ys.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(420, false),
        RT::on_completed::<bool>(420),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn sequence_equal_observable_not_equal_3_sym() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_completed::<i32>(330),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(400, 4),
        RT::on_completed::<i32>(420),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || ys.as_observable().sequence_equal(xs.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(420, false),
        RT::on_completed::<bool>(420),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn sequence_equal_observable_comparer_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_completed::<i32>(330),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(400, 4),
        RT::on_completed::<i32>(420),
    ]);

    let ex = new_error("ex");

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_sequence_equal_with(ys.as_observable(), move |_, _| Err(ex.clone()))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(270, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 270));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 270));
}

#[test]
fn sequence_equal_observable_comparer_throws_sym() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_completed::<i32>(330),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(270, 3),
        RT::on_next(400, 4),
        RT::on_completed::<i32>(420),
    ]);

    let ex = new_error("ex");

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        let ex = ex.clone();
        move || ys.as_observable().try_sequence_equal_with(xs.as_observable(), move |_, _| Err(ex.clone()))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(270, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 270));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 270));
}

#[test]
fn sequence_equal_observable_not_equal_4() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(250, 1),
        RT::on_completed::<i32>(300),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(290, 1),
        RT::on_next(310, 2),
        RT::on_completed::<i32>(350),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || xs.as_observable().sequence_equal(ys.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, false),
        RT::on_completed::<bool>(310),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn sequence_equal_observable_not_equal_4_sym() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(250, 1),
        RT::on_completed::<i32>(300),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(290, 1),
        RT::on_next(310, 2),
        RT::on_completed::<i32>(350),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || ys.as_observable().sequence_equal(xs.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, false),
        RT::on_completed::<bool>(310),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn sequence_equal_observable_left_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(250, 1),
        RT::on_error::<i32>(300, ex.clone()),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(290, 1),
        RT::on_next(310, 2),
        RT::on_completed::<i32>(350),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || ys.as_observable().sequence_equal(xs.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(300, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn sequence_equal_observable_right_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(290, 1),
        RT::on_next(310, 2),
        RT::on_completed::<i32>(350),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        RT::on_next(250, 1),
        RT::on_error::<i32>(300, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ys = ys.clone();
        move || ys.as_observable().sequence_equal(xs.as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(300, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn sequence_equal_enumerable_equal() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sequence_equal_iter(vec![3, 4, 5, 6, 7])
    });

    assert_messages!(
        res.messages(),
        RT::on_next(510, true),
        RT::on_completed::<bool>(510),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 510));
}

#[test]
fn sequence_equal_enumerable_not_equal_elements() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sequence_equal_iter(vec![3, 4, 9, 6, 7])
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, false),
        RT::on_completed::<bool>(310),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn sequence_equal_enumerable_comparer_equal() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sequence_equal_iter_with(vec![3 - 2, 4, 5, 6 + 42, 7 - 6], odd_even)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(510, true),
        RT::on_completed::<bool>(510),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 510));
}

#[test]
fn sequence_equal_enumerable_comparer_not_equal() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sequence_equal_iter_with(vec![3 - 2, 4, 5 + 9, 6 + 42, 7 - 6], odd_even)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, false),
        RT::on_completed::<bool>(310),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn sequence_equal_enumerable_comparer_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let ex = new_error("ex");

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_sequence_equal_iter_with(|| Ok(vec![3, 4, 5, 6, 7].into_iter().map(Ok)), move |x, y| if *x == 5 { Err(ex.clone()) } else { Ok(x == y) })
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(310, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn sequence_equal_enumerable_not_equal_too_long() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sequence_equal_iter(vec![3, 4, 5, 6, 7, 8])
    });

    assert_messages!(
        res.messages(),
        RT::on_next(510, false),
        RT::on_completed::<bool>(510),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 510));
}

#[test]
fn sequence_equal_enumerable_not_equal_too_short() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(310, 5),
        RT::on_next(340, 6),
        RT::on_next(450, 7),
        RT::on_completed::<i32>(510),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sequence_equal_iter(vec![3, 4, 5, 6])
    });

    assert_messages!(
        res.messages(),
        RT::on_next(450, false),
        RT::on_completed::<bool>(450),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn sequence_equal_enumerable_on_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_error::<i32>(310, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().sequence_equal_iter(vec![3, 4])
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(310, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn sequence_equal_enumerable_iterator_throws1() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_completed::<i32>(310),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_sequence_equal_iter_with(move || Ok(throw_after_3(&ex)), |x, y| Ok(x == y))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(290, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 290));
}

#[test]
fn sequence_equal_enumerable_iterator_throws2() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_completed::<i32>(310),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_sequence_equal_iter_with(move || Ok(throw_after_3(&ex)), |x, y| Ok(x == y))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(310, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn sequence_equal_enumerable_get_enumerator_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(190, 2),
        RT::on_next(240, 3),
        RT::on_completed::<i32>(310),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_sequence_equal_iter_with(move || Err::<Vec<Result<i32, ObservableError>>, _>(ex.clone()), |x, y| Ok(x == y))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<bool>(200, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions());
}

// Not ported: SequenceEqual_ArgumentChecking - argument validation
