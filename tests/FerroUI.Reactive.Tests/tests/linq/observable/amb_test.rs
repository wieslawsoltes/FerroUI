//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AmbTest.cs` of Rx.NET.
//! Adapted: `Observable.Amb(params)` and `Amb` of an enumerable are both `Observable::amb_all`
//! (the array tests and the enumerable tests run the same port).

use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn amb_never2() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let r = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b) = (l.as_observable(), r.as_observable());
    let res = scheduler.start(move || a.amb(b));

    assert_messages!(res.messages());

    assert_subscriptions!(l.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(r.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn amb_never3() {
    let scheduler = TestScheduler::new();

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let n2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let n3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = vec![n1.as_observable(), n2.as_observable(), n3.as_observable()];
    let res = scheduler.start(move || Observable::amb_all(sources));

    assert_messages!(res.messages());

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(n2.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(n3.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn amb_never3_params() {
    let scheduler = TestScheduler::new();

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let n2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let n3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [n1.as_observable(), n2.as_observable(), n3.as_observable()];
    let res = scheduler.start(move || Observable::amb_all(sources));

    assert_messages!(res.messages());

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(n2.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(n3.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn amb_never_empty() {
    let scheduler = TestScheduler::new();

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(225)]);

    let (a, b) = (n.as_observable(), e.as_observable());
    let res = scheduler.start(move || a.amb(b));

    assert_messages!(res.messages(), RT::on_completed::<i32>(225));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 225));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 225));
}

#[test]
fn amb_empty_never() {
    let scheduler = TestScheduler::new();

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(225)]);

    let (a, b) = (e.as_observable(), n.as_observable());
    let res = scheduler.start(move || a.amb(b));

    assert_messages!(res.messages(), RT::on_completed::<i32>(225));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 225));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 225));
}

#[test]
fn amb_regular_should_dispose_loser() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(240)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.amb(b));

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(240));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 240));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn amb_winner_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(220, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.amb(b));

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn amb_loser_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_error(230, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 3), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.amb(b));

    assert_messages!(res.messages(), RT::on_next(210, 3), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn amb_throws_before_election_left() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(210, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.amb(b));

    assert_messages!(res.messages(), RT::on_error::<i32>(210, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn amb_throws_before_election_right() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(210, ex.clone())]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.amb(b));

    assert_messages!(res.messages(), RT::on_error::<i32>(210, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn amb_many_array_on_next() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_cold_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let o2 = scheduler.create_cold_observable(vec![RT::on_next(150, 2), RT::on_error(210, ex.clone())]);

    let o3 = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(150)]);

    let sources = [o1.as_observable(), o2.as_observable(), o3.as_observable()];
    let res = scheduler.start(move || Observable::amb_all(sources));

    assert_messages!(res.messages(), RT::on_next(350, 1), RT::on_next(420, 3), RT::on_completed(450));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 450));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(200, 350));
}

#[test]
fn amb_many_array_on_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_cold_observable(vec![RT::on_error::<i32>(150, ex.clone())]);

    let o2 = scheduler.create_cold_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let o3 = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(150)]);

    let sources = [o1.as_observable(), o2.as_observable(), o3.as_observable()];
    let res = scheduler.start(move || Observable::amb_all(sources));

    assert_messages!(res.messages(), RT::on_error::<i32>(350, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(200, 350));
}

#[test]
fn amb_many_array_on_completed() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(150)]);

    let o2 = scheduler.create_cold_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let o3 = scheduler.create_cold_observable(vec![RT::on_next(150, 2), RT::on_error(210, ex.clone())]);

    let sources = [o1.as_observable(), o2.as_observable(), o3.as_observable()];
    let res = scheduler.start(move || Observable::amb_all(sources));

    assert_messages!(res.messages(), RT::on_completed::<i32>(350));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(200, 350));
}

#[test]
fn amb_many_enumerable_on_next() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_cold_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let o2 = scheduler.create_cold_observable(vec![RT::on_next(150, 2), RT::on_error(210, ex.clone())]);

    let o3 = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(150)]);

    let sources = vec![o1.as_observable(), o2.as_observable(), o3.as_observable()];
    let res = scheduler.start(move || Observable::amb_all(sources));

    assert_messages!(res.messages(), RT::on_next(350, 1), RT::on_next(420, 3), RT::on_completed(450));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 450));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(200, 350));
}

#[test]
fn amb_many_enumerable_on_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_cold_observable(vec![RT::on_error::<i32>(150, ex.clone())]);

    let o2 = scheduler.create_cold_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let o3 = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(150)]);

    let sources = vec![o1.as_observable(), o2.as_observable(), o3.as_observable()];
    let res = scheduler.start(move || Observable::amb_all(sources));

    assert_messages!(res.messages(), RT::on_error::<i32>(350, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(200, 350));
}

#[test]
fn amb_many_enumerable_on_completed() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(150)]);

    let o2 = scheduler.create_cold_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(250)]);

    let o3 = scheduler.create_cold_observable(vec![RT::on_next(150, 2), RT::on_error(210, ex.clone())]);

    let sources = vec![o1.as_observable(), o2.as_observable(), o3.as_observable()];
    let res = scheduler.start(move || Observable::amb_all(sources));

    assert_messages!(res.messages(), RT::on_completed::<i32>(350));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(200, 350));
}

// Not ported: Amb_ArgumentChecking - argument validation
// Not ported: Amb_Many_Enumerable_Many_Sources - blocking operator First (waived: one thread)
// Not ported: Amb_Many_Enumerable_Many_Sources_NoStackOverflow - blocking operator First (waived: one thread)
