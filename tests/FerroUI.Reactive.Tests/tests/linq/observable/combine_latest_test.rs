//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/CombineLatestTest.cs` of Rx.NET: the tests
//! of two sources, of a list of sources and `AtLeastOneThrows4`; the tests of each number of sources
//! (regions Never, Empty, SelectorThrows, AllEmptyButOne and Typical) are in
//! `combine_latest_arity_test.rs`.
//! Adapted: `x.CombineLatest(y, selector)` is `combine_latest`, `Observable.CombineLatest(e0, e1, e2,
//! e3, selector)` is `combine_latest4` on the first source, and the list forms (`IEnumerable` and
//! `params`) are `Observable::combine_latest_all` (a `Vec` for upstream's `IList`) and
//! `Observable::combine_latest_all_with` (a slice for the selector's `IList`). The selector that
//! throws is the selector of `try_combine_latest` returning the error.

use ferroui_reactive::{Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn combine_latest_never_empty() {
    let scheduler = TestScheduler::new();

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let (left, right) = (n.as_observable(), e.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn combine_latest_empty_never() {
    let scheduler = TestScheduler::new();

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (left, right) = (e.as_observable(), n.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn combine_latest_empty_return() {
    let scheduler = TestScheduler::new();

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let (left, right) = (e.as_observable(), o.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_completed(215));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn combine_latest_return_empty() {
    let scheduler = TestScheduler::new();

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let (left, right) = (o.as_observable(), e.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_completed(215));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn combine_latest_never_return() {
    let scheduler = TestScheduler::new();

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (left, right) = (n.as_observable(), o.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn combine_latest_return_never() {
    let scheduler = TestScheduler::new();

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (left, right) = (o.as_observable(), n.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn combine_latest_return_return() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(240)]);

    let (left, right) = (o1.as_observable(), o2.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_next(220, 2 + 3), RT::on_completed(240));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn combine_latest_empty_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (left, right) = (o1.as_observable(), o2.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_error_empty() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let f = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (left, right) = (f.as_observable(), e.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(f.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_return_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (left, right) = (o1.as_observable(), o2.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_throw_return() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_throw_throw() {
    let scheduler = TestScheduler::new();

    let ex1 = new_error("ex1");
    let ex2 = new_error("ex2");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex1.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex2.clone())]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex1.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_error_throw() {
    let scheduler = TestScheduler::new();

    let ex1 = new_error("ex1");
    let ex2 = new_error("ex2");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(220, ex1.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex2.clone())]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex1.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_throw_error() {
    let scheduler = TestScheduler::new();

    let ex1 = new_error("ex1");
    let ex2 = new_error("ex2");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(220, ex1.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex2.clone())]);

    let (left, right) = (o1.as_observable(), o2.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex1.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_never_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (left, right) = (o1.as_observable(), o2.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_throw_never() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_some_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (left, right) = (o1.as_observable(), o2.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_throw_some() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_throw_after_complete_left() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex.clone())]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(230, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn combine_latest_throw_after_complete_right() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex.clone())]);

    let (left, right) = (o1.as_observable(), o2.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(230, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn combine_latest_interleaved_with_tail() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 3),
        RT::on_next(230, 5),
        RT::on_next(235, 6),
        RT::on_next(240, 7),
        RT::on_completed(250),
    ]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(
        res.messages(),
        RT::on_next(220, 2 + 3),
        RT::on_next(225, 3 + 4),
        RT::on_next(230, 4 + 5),
        RT::on_next(235, 4 + 6),
        RT::on_next(240, 4 + 7),
        RT::on_completed(250)
    );

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn combine_latest_consecutive() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(235, 6), RT::on_next(240, 7), RT::on_completed(250)]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_next(235, 4 + 6), RT::on_next(240, 4 + 7), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn combine_latest_consecutive_end_with_error_left() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(215, 2),
        RT::on_next(225, 4),
        RT::on_error(230, ex.clone()),
    ]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(235, 6), RT::on_next(240, 7), RT::on_completed(250)]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(230, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn combine_latest_consecutive_end_with_error_right() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(250)]);

    let o2 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(235, 6),
        RT::on_next(240, 7),
        RT::on_error(245, ex.clone()),
    ]);

    let (left, right) = (o2.as_observable(), o1.as_observable());

    let res = scheduler.start(move || left.combine_latest(right, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_next(235, 4 + 6), RT::on_next(240, 4 + 7), RT::on_error(245, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 245));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 245));
}

#[test]
fn combine_latest_selector_throws() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(240)]);

    let ex = new_error("ex");

    let (left, right) = (o2.as_observable(), o1.as_observable());
    let selector_error = ex.clone();

    let res = scheduler.start(move || left.try_combine_latest(right, move |_x: i32, _y: i32| -> Result<i32, ObservableError> { Err(selector_error.clone()) }));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn combine_latest_list_regular() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(240, 4), RT::on_completed(270)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(250, 5), RT::on_completed(280)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(290)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all(sources));

    assert_messages!(
        res.messages(),
        RT::on_next_where(230, |l: &Vec<i32>| l.as_slice() == [1, 2, 3]),
        RT::on_next_where(240, |l: &Vec<i32>| l.as_slice() == [4, 2, 3]),
        RT::on_next_where(250, |l: &Vec<i32>| l.as_slice() == [4, 5, 3]),
        RT::on_next_where(260, |l: &Vec<i32>| l.as_slice() == [4, 5, 6]),
        RT::on_completed(290)
    );
}

#[test]
fn combine_latest_n_ary_symmetric() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(250, 4), RT::on_completed(420)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(240, 5), RT::on_completed(410)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(400)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all(sources));

    assert_messages!(
        res.messages(),
        RT::on_next_where(230, |l: &Vec<i32>| l.as_slice() == [1, 2, 3]),
        RT::on_next_where(240, |l: &Vec<i32>| l.as_slice() == [1, 5, 3]),
        RT::on_next_where(250, |l: &Vec<i32>| l.as_slice() == [4, 5, 3]),
        RT::on_next_where(260, |l: &Vec<i32>| l.as_slice() == [4, 5, 6]),
        RT::on_completed(420)
    );

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 420));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn combine_latest_n_ary_symmetric_selector() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(250, 4), RT::on_completed(420)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(240, 5), RT::on_completed(410)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(400)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all_with(sources, |xs: &[i32]| xs.iter().sum::<i32>()));

    assert_messages!(
        res.messages(),
        RT::on_next(230, [1, 2, 3].iter().sum::<i32>()),
        RT::on_next(240, [1, 5, 3].iter().sum::<i32>()),
        RT::on_next(250, [4, 5, 3].iter().sum::<i32>()),
        RT::on_next(260, [4, 5, 6].iter().sum::<i32>()),
        RT::on_completed(420)
    );

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 420));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn combine_latest_n_ary_asymmetric() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(250, 4), RT::on_completed(270)]);
    let e1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 5),
        RT::on_next(290, 7),
        RT::on_next(310, 9),
        RT::on_completed(410),
    ]);
    let e2 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(230, 3),
        RT::on_next(260, 6),
        RT::on_next(280, 8),
        RT::on_completed(300),
    ]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all(sources));

    assert_messages!(
        res.messages(),
        RT::on_next_where(230, |l: &Vec<i32>| l.as_slice() == [1, 2, 3]),
        RT::on_next_where(240, |l: &Vec<i32>| l.as_slice() == [1, 5, 3]),
        RT::on_next_where(250, |l: &Vec<i32>| l.as_slice() == [4, 5, 3]),
        RT::on_next_where(260, |l: &Vec<i32>| l.as_slice() == [4, 5, 6]),
        RT::on_next_where(280, |l: &Vec<i32>| l.as_slice() == [4, 5, 8]),
        RT::on_next_where(290, |l: &Vec<i32>| l.as_slice() == [4, 7, 8]),
        RT::on_next_where(310, |l: &Vec<i32>| l.as_slice() == [4, 9, 8]),
        RT::on_completed(410)
    );

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 270));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn combine_latest_n_ary_asymmetric_selector() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(250, 4), RT::on_completed(270)]);
    let e1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 5),
        RT::on_next(290, 7),
        RT::on_next(310, 9),
        RT::on_completed(410),
    ]);
    let e2 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(230, 3),
        RT::on_next(260, 6),
        RT::on_next(280, 8),
        RT::on_completed(300),
    ]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all_with(sources, |xs: &[i32]| xs.iter().sum::<i32>()));

    assert_messages!(
        res.messages(),
        RT::on_next(230, [1, 2, 3].iter().sum::<i32>()),
        RT::on_next(240, [1, 5, 3].iter().sum::<i32>()),
        RT::on_next(250, [4, 5, 3].iter().sum::<i32>()),
        RT::on_next(260, [4, 5, 6].iter().sum::<i32>()),
        RT::on_next(280, [4, 5, 8].iter().sum::<i32>()),
        RT::on_next(290, [4, 7, 8].iter().sum::<i32>()),
        RT::on_next(310, [4, 9, 8].iter().sum::<i32>()),
        RT::on_completed(410)
    );

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 270));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn combine_latest_n_ary_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_error(250, ex.clone())]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(240, 5), RT::on_completed(410)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(400)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all(sources));

    assert_messages!(
        res.messages(),
        RT::on_next_where(230, |l: &Vec<i32>| l.as_slice() == [1, 2, 3]),
        RT::on_next_where(240, |l: &Vec<i32>| l.as_slice() == [1, 5, 3]),
        RT::on_error(250, ex.clone())
    );

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn combine_latest_n_ary_error_selector() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_error(250, ex.clone())]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(240, 5), RT::on_completed(410)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(400)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all_with(sources, |xs: &[i32]| xs.iter().sum::<i32>()));

    assert_messages!(
        res.messages(),
        RT::on_next(230, [1, 2, 3].iter().sum::<i32>()),
        RT::on_next(240, [1, 5, 3].iter().sum::<i32>()),
        RT::on_error(250, ex.clone())
    );

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn combine_latest_at_least_one_throws4() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();
    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex.clone())]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.combine_latest4(s1, s2, s3, |_0, _1, _2, _3| 42)
    });

    assert_messages!(res.messages(), RT::on_error(230, ex.clone()));

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 230));
    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 230));
    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 230));
    assert_subscriptions!(e3.subscriptions(), RT::subscribe(200, 230));
}

// Not ported: CombineLatest_ArgumentChecking - argument validation
// Not ported: CombineLatest_ArgumentCheckingHighArity - argument validation
// Not ported: CombineLatest_NAry_ArgumentChecking - argument validation
