//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/CaseTest.cs` of Rx.NET.
//! Adapted: `Case(selector, sources, defaultSource)` is `Observable::case_with_default`,
//! `Case(selector, sources, scheduler)` is `Observable::case_on` and `Case(selector, sources)` is
//! `Observable::case`; the dictionary is a `HashMap`; a selector that throws is the `try_` form
//! returning the error.

use ferroui_reactive::{IObservable, Observable};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, HotObservable, ReactiveTest as RT, TestScheduler};
use std::collections::HashMap;
use std::rc::Rc;

fn map_of(xs: &Rc<HotObservable<i32>>, ys: &Rc<HotObservable<i32>>) -> HashMap<i32, Rc<dyn IObservable<i32>>> {
    let mut map: HashMap<i32, Rc<dyn IObservable<i32>>> = HashMap::new();
    map.insert(1, xs.as_observable());
    map.insert(2, ys.as_observable());
    map
}

fn sources(scheduler: &TestScheduler) -> (Rc<HotObservable<i32>>, Rc<HotObservable<i32>>, Rc<HotObservable<i32>>) {
    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(240, 2), RT::on_next(270, 3), RT::on_completed(300)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(220, 11), RT::on_next(250, 12), RT::on_next(280, 13), RT::on_completed(310)]);

    let zs = scheduler.create_hot_observable(vec![RT::on_next(230, 21), RT::on_next(240, 22), RT::on_next(290, 23), RT::on_completed(320)]);

    (xs, ys, zs)
}

#[test]
fn case_one() {
    let scheduler = TestScheduler::new();

    let (xs, ys, zs) = sources(&scheduler);

    let map = map_of(&xs, &ys);

    let z = zs.as_observable();
    let results = scheduler.start(move || Observable::case_with_default(|| 1, map, z));

    assert_messages!(results.messages(), RT::on_next(210, 1), RT::on_next(240, 2), RT::on_next(270, 3), RT::on_completed(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions());

    assert_subscriptions!(zs.subscriptions());
}

#[test]
fn case_two() {
    let scheduler = TestScheduler::new();

    let (xs, ys, zs) = sources(&scheduler);

    let map = map_of(&xs, &ys);

    let z = zs.as_observable();
    let results = scheduler.start(move || Observable::case_with_default(|| 2, map, z));

    assert_messages!(results.messages(), RT::on_next(220, 11), RT::on_next(250, 12), RT::on_next(280, 13), RT::on_completed(310));

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(zs.subscriptions());
}

#[test]
fn case_three() {
    let scheduler = TestScheduler::new();

    let (xs, ys, zs) = sources(&scheduler);

    let map = map_of(&xs, &ys);

    let z = zs.as_observable();
    let results = scheduler.start(move || Observable::case_with_default(|| 3, map, z));

    assert_messages!(results.messages(), RT::on_next(230, 21), RT::on_next(240, 22), RT::on_next(290, 23), RT::on_completed(320));

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions());

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(200, 320));
}

#[test]
fn case_throw() {
    let scheduler = TestScheduler::new();

    let (xs, ys, zs) = sources(&scheduler);

    let map = map_of(&xs, &ys);

    let ex = new_error("ex");

    let (z, e) = (zs.as_observable(), ex.clone());
    let results = scheduler.start(move || Observable::try_case_with_default(move || Err(e.clone()), map, z));

    assert_messages!(results.messages(), RT::on_error::<i32>(200, ex));

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions());

    assert_subscriptions!(zs.subscriptions());
}

#[test]
fn case_with_default_one() {
    let scheduler = TestScheduler::new();

    let (xs, ys, _) = sources(&scheduler);

    let map = map_of(&xs, &ys);

    let s = scheduler.scheduler();
    let results = scheduler.start(move || Observable::case_on(|| 1, map, s));

    assert_messages!(results.messages(), RT::on_next(210, 1), RT::on_next(240, 2), RT::on_next(270, 3), RT::on_completed(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn case_with_default_two() {
    let scheduler = TestScheduler::new();

    let (xs, ys, _) = sources(&scheduler);

    let map = map_of(&xs, &ys);

    let s = scheduler.scheduler();
    let results = scheduler.start(move || Observable::case_on(|| 2, map, s));

    assert_messages!(results.messages(), RT::on_next(220, 11), RT::on_next(250, 12), RT::on_next(280, 13), RT::on_completed(310));

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn case_with_default_three() {
    let scheduler = TestScheduler::new();

    let (xs, ys, _) = sources(&scheduler);

    let map = map_of(&xs, &ys);

    let s = scheduler.scheduler();
    let results = scheduler.start(move || Observable::case_on(|| 3, map, s));

    assert_messages!(results.messages(), RT::on_completed::<i32>(201));

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn case_with_default_throw() {
    let scheduler = TestScheduler::new();

    let (xs, ys, _) = sources(&scheduler);

    let map = map_of(&xs, &ys);

    let ex = new_error("ex");

    let (s, e) = (scheduler.scheduler(), ex.clone());
    let results = scheduler.start(move || Observable::try_case_on(move || Err(e.clone()), map, s));

    assert_messages!(results.messages(), RT::on_error::<i32>(200, ex));

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions());
}

#[test]
fn case_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(240, 2), RT::on_next(270, 3), RT::on_error(300, ex.clone())]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(220, 11), RT::on_next(250, 12), RT::on_next(280, 13), RT::on_completed(310)]);

    let map = map_of(&xs, &ys);

    let s = scheduler.scheduler();
    let results = scheduler.start(move || Observable::case_on(|| 1, map, s));

    assert_messages!(results.messages(), RT::on_next(210, 1), RT::on_next(240, 2), RT::on_next(270, 3), RT::on_error(300, ex));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions());
}

// Not ported: Case_ArgumentChecking - argument validation
// Not ported: CaseWithDefault_CheckDefault - real threads (DefaultScheduler.Instance, compared through the blocking ToEnumerable of AssertEqual)
