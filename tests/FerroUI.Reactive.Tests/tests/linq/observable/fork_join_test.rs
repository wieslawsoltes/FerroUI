//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ForkJoinTest.cs` of Rx.NET.
//! Adapted: `first.ForkJoin(second, selector)` is the method `fork_join`, a selector that throws
//! the `try_fork_join` form returning the error; `ObservableEx.ForkJoin(params sources)` and
//! `ForkJoin(IEnumerable<IObservable<T>>)` are `Observable::fork_join_all`, whose arrays are `Vec`s;
//! `ForkJoin_Nary_Immediate` reads the array with a subscription, which delivers it before it
//! returns (`Return` notifies at once; upstream's blocking `First` waits for it).

use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn fork_join_empty_empty() {
    let scheduler = TestScheduler::new();

    let msgs1 = vec![
        RT::on_next(150, 1),
        RT::on_completed(230),
    ];

    let msgs2 = vec![
        RT::on_next(150, 1),
        RT::on_completed(250),
    ];

    let o = scheduler.create_hot_observable(msgs1);
    let e = scheduler.create_hot_observable(msgs2);

    let (x, y) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || x.fork_join(y, |x, y| x + y));
    assert_messages!(
        res.messages(),
        RT::on_completed(250)
    );
}

#[test]
fn fork_join_none() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| Observable::fork_join_all(Vec::<Rc<dyn IObservable<i32>>>::new()));
    assert_messages!(res.messages(), RT::on_completed::<Vec<i32>>(200));
}

#[test]
fn fork_join_empty_return() {
    let scheduler = TestScheduler::new();

    let msgs1 = vec![
        RT::on_next(150, 1),
        RT::on_completed(230),
    ];

    let msgs2 = vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed(250),
    ];

    let o = scheduler.create_hot_observable(msgs1);
    let e = scheduler.create_hot_observable(msgs2);

    let (x, y) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || x.fork_join(y, |x, y| x + y));
    assert_messages!(
        res.messages(),
        RT::on_completed(250)
    );
}

#[test]
fn fork_join_return_empty() {
    let scheduler = TestScheduler::new();

    let msgs1 = vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed(230),
    ];

    let msgs2 = vec![
        RT::on_next(150, 1),
        RT::on_completed(250),
    ];

    let o = scheduler.create_hot_observable(msgs1);
    let e = scheduler.create_hot_observable(msgs2);

    let (x, y) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || x.fork_join(y, |x, y| x + y));
    assert_messages!(
        res.messages(),
        RT::on_completed(250)
    );
}

#[test]
fn fork_join_return_return() {
    let scheduler = TestScheduler::new();

    let msgs1 = vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed(230),
    ];

    let msgs2 = vec![
        RT::on_next(150, 1),
        RT::on_next(220, 3),
        RT::on_completed(250),
    ];

    let o = scheduler.create_hot_observable(msgs1);
    let e = scheduler.create_hot_observable(msgs2);

    let (x, y) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || x.fork_join(y, |x, y| x + y));
    assert_messages!(
        res.messages(),
        RT::on_next(250, 2 + 3),
        RT::on_completed(250)
    );
}

#[test]
fn fork_join_empty_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let msgs1 = vec![
        RT::on_next(150, 1),
        RT::on_completed(230),
    ];

    let msgs2 = vec![
        RT::on_next(150, 1),
        RT::on_error(210, ex.clone()),
        RT::on_completed(250),
    ];

    let o = scheduler.create_hot_observable(msgs1);
    let e = scheduler.create_hot_observable(msgs2);

    let (x, y) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || x.fork_join(y, |x, y| x + y));
    assert_messages!(
        res.messages(),
        RT::on_error(210, ex.clone())
    );
}

#[test]
fn fork_join_throw_empty() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let msgs1 = vec![
        RT::on_next(150, 1),
        RT::on_error(210, ex.clone()),
        RT::on_completed(230),
    ];

    let msgs2 = vec![
        RT::on_next(150, 1),
        RT::on_completed(250),
    ];

    let o = scheduler.create_hot_observable(msgs1);
    let e = scheduler.create_hot_observable(msgs2);

    let (x, y) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || x.fork_join(y, |x, y| x + y));
    assert_messages!(
        res.messages(),
        RT::on_error(210, ex.clone())
    );
}

#[test]
fn fork_join_return_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let msgs1 = vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed(230),
    ];

    let msgs2 = vec![
        RT::on_next(150, 1),
        RT::on_error(220, ex.clone()),
        RT::on_completed(250),
    ];

    let o = scheduler.create_hot_observable(msgs1);
    let e = scheduler.create_hot_observable(msgs2);

    let (x, y) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || x.fork_join(y, |x, y| x + y));
    assert_messages!(
        res.messages(),
        RT::on_error(220, ex.clone())
    );
}

#[test]
fn fork_join_throw_return() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let msgs1 = vec![
        RT::on_next(150, 1),
        RT::on_error(220, ex.clone()),
        RT::on_completed(230),
    ];

    let msgs2 = vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_completed(250),
    ];

    let o = scheduler.create_hot_observable(msgs1);
    let e = scheduler.create_hot_observable(msgs2);

    let (x, y) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || x.fork_join(y, |x, y| x + y));
    assert_messages!(
        res.messages(),
        RT::on_error(220, ex.clone())
    );
}

#[test]
fn fork_join_binary() {
    let scheduler = TestScheduler::new();

    let msgs1 = vec![
        RT::on_next(150, 1),
        RT::on_next(215, 2),
        RT::on_next(225, 4),
        RT::on_completed(230),
    ];

    let msgs2 = vec![
        RT::on_next(150, 1),
        RT::on_next(235, 6),
        RT::on_next(240, 7),
        RT::on_completed(250),
    ];

    let o = scheduler.create_hot_observable(msgs1);
    let e = scheduler.create_hot_observable(msgs2);

    let (x, y) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || x.fork_join(y, |x, y| x + y));
    assert_messages!(
        res.messages(),
        RT::on_next(250, 4 + 7),
        RT::on_completed(250)
    );
}

/// The three sources of the n-ary tests.
fn nary_sources(scheduler: &TestScheduler) -> (Rc<dyn IObservable<i32>>, Rc<dyn IObservable<i32>>, Rc<dyn IObservable<i32>>) {
    let msgs1 = vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(230)];

    let msgs2 = vec![RT::on_next(150, 1), RT::on_next(235, 6), RT::on_next(240, 7), RT::on_completed(250)];

    let msgs3 = vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(245, 5), RT::on_completed(270)];

    let o1 = scheduler.create_hot_observable(msgs1);
    let o2 = scheduler.create_hot_observable(msgs2);
    let o3 = scheduler.create_hot_observable(msgs3);

    (o1.as_observable(), o2.as_observable(), o3.as_observable())
}

#[test]
fn fork_join_nary_params() {
    let scheduler = TestScheduler::new();

    let (o1, o2, o3) = nary_sources(&scheduler);

    let res = scheduler.start(move || Observable::fork_join_all(vec![o1, o2, o3]));

    assert_messages!(
        res.messages(),
        RT::on_next_where(270, |l: &Vec<i32>| *l == vec![4, 7, 5]),
        RT::on_completed(270)
    );
}

#[test]
fn fork_join_nary_params_empty() {
    let scheduler = TestScheduler::new();

    let msgs1 = vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(230)];

    let msgs2 = vec![RT::on_next(150, 1), RT::on_next(235, 6), RT::on_next(240, 7), RT::on_completed(250)];

    let msgs3 = vec![RT::on_completed::<i32>(270)];

    let o1 = scheduler.create_hot_observable(msgs1).as_observable();
    let o2 = scheduler.create_hot_observable(msgs2).as_observable();
    let o3 = scheduler.create_hot_observable(msgs3).as_observable();

    let res = scheduler.start(move || Observable::fork_join_all(vec![o1, o2, o3]));

    assert_messages!(res.messages(), RT::on_completed::<Vec<i32>>(270));
}

#[test]
fn fork_join_nary_params_empty_before_end() {
    let scheduler = TestScheduler::new();

    let msgs1 = vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(230)];

    let msgs2 = vec![RT::on_completed::<i32>(235)];

    let msgs3 = vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(245, 5), RT::on_completed(270)];

    let o1 = scheduler.create_hot_observable(msgs1).as_observable();
    let o2 = scheduler.create_hot_observable(msgs2).as_observable();
    let o3 = scheduler.create_hot_observable(msgs3).as_observable();

    let res = scheduler.start(move || Observable::fork_join_all(vec![o1, o2, o3]));

    assert_messages!(res.messages(), RT::on_completed::<Vec<i32>>(235));
}

#[test]
fn fork_join_nary_immediate() {
    let result = Rc::new(RefCell::new(None));
    let target = result.clone();
    let _d = Observable::fork_join_all(vec![Observable::return_(1), Observable::return_(2)])
        .first_async()
        .subscribe_next(move |values| *target.borrow_mut() = Some(values));

    assert_eq!(Some(vec![1, 2]), result.borrow().clone());
}

#[test]
fn fork_join_nary_virtual_and_immediate() {
    let scheduler = TestScheduler::new();

    let (o1, o2, o3) = nary_sources(&scheduler);

    let res = scheduler.start(move || Observable::fork_join_all(vec![o1, o2, o3, Observable::return_(20)]));

    assert_messages!(
        res.messages(),
        RT::on_next_where(270, |l: &Vec<i32>| *l == vec![4, 7, 5, 20]),
        RT::on_completed(270)
    );
}

#[test]
fn fork_join_nary_immediate_and_virtual() {
    let scheduler = TestScheduler::new();

    let (o1, o2, o3) = nary_sources(&scheduler);

    let res = scheduler.start(move || Observable::fork_join_all(vec![Observable::return_(20), o1, o2, o3]));

    assert_messages!(
        res.messages(),
        RT::on_next_where(270, |l: &Vec<i32>| *l == vec![20, 4, 7, 5]),
        RT::on_completed(270)
    );
}

#[test]
fn fork_join_nary() {
    let scheduler = TestScheduler::new();

    let (o1, o2, o3) = nary_sources(&scheduler);

    let res = scheduler.start(move || Observable::fork_join_all(vec![o1, o2, o3]));

    assert_messages!(
        res.messages(),
        RT::on_next_where(270, |l: &Vec<i32>| *l == vec![4, 7, 5]),
        RT::on_completed(270)
    );
}

#[test]
fn bug_1302_selector_throws_left_last() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_completed(220)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(215, 2), RT::on_completed(217)]);

    let ex = new_error("ex");

    let (x, y, e) = (xs.as_observable(), ys.as_observable(), ex.clone());
    let results = scheduler.start(move || x.try_fork_join(y, move |_x, _y| Err::<i32, _>(e.clone())));

    assert_messages!(results.messages(), RT::on_error::<i32>(220, ex));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 217));
}

#[test]
fn bug_1302_selector_throws_right_last() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_completed(217)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(215, 2), RT::on_completed(220)]);

    let ex = new_error("ex");

    let (x, y, e) = (xs.as_observable(), ys.as_observable(), ex.clone());
    let results = scheduler.start(move || x.try_fork_join(y, move |_x, _y| Err::<i32, _>(e.clone())));

    assert_messages!(results.messages(), RT::on_error::<i32>(220, ex));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 217));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn bug_1302_right_last_no_left() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(217)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(215, 2), RT::on_completed(220)]);

    let (x, y) = (xs.as_observable(), ys.as_observable());
    let results = scheduler.start(move || x.fork_join(y, |x, y| x + y));

    assert_messages!(results.messages(), RT::on_completed(220));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 217));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn bug_1302_right_last_no_right() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(215, 2), RT::on_completed(217)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(220)]);

    let (x, y) = (xs.as_observable(), ys.as_observable());
    let results = scheduler.start(move || x.fork_join(y, |x, y| x + y));

    assert_messages!(results.messages(), RT::on_completed(220));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 217));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 220));
}

// Not ported: ForkJoin_ArgumentChecking - argument validation
