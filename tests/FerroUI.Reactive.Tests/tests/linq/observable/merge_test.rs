//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/MergeTest.cs` of Rx.NET.
//! Adapted: `Observable.Merge(scheduler, xs, ys)` and `new[] { xs, ys }.Merge(scheduler)` are
//! `Observable::merge_all_on`, `Observable.Merge(xs, ys, scheduler)` is `merge_on`, `Merge()` and
//! `Merge(int)` on a sequence of sequences are `merge_inner` and `merge_inner_max`; the inner
//! sequences of a sequence of sequences are `Rc<dyn IObservable<i32>>` handles of the test
//! observables.

use ferroui_reactive::{IObservable, Notification, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, Recorded, TestScheduler};
use std::rc::Rc;

type Inner = Rc<dyn IObservable<i32>>;

#[test]
fn merge_never2() {
    let scheduler = TestScheduler::new();

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let n2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b, s) = (n1.as_observable(), n2.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages());

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(201, 1000));

    assert_subscriptions!(n2.subscriptions(), RT::subscribe(202, 1000));
}

#[test]
fn merge_never3() {
    let scheduler = TestScheduler::new();

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let n2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let n3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b, c, s) = (n1.as_observable(), n2.as_observable(), n3.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b, c], s));

    assert_messages!(res.messages());

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(201, 1000));

    assert_subscriptions!(n2.subscriptions(), RT::subscribe(202, 1000));

    assert_subscriptions!(n3.subscriptions(), RT::subscribe(203, 1000));
}

#[test]
fn merge_empty2() {
    let scheduler = TestScheduler::new();

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let (a, b, s) = (e1.as_observable(), e2.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(230));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(201, 210));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(202, 230));
}

#[test]
fn merge_empty3() {
    let scheduler = TestScheduler::new();

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);

    let (a, b, c, s) = (e1.as_observable(), e2.as_observable(), e3.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b, c], s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(240));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(201, 210));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(202, 230));

    assert_subscriptions!(e3.subscriptions(), RT::subscribe(203, 240));
}

#[test]
fn merge_empty_delayed2_right_last() {
    let scheduler = TestScheduler::new();

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);

    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let (a, b, s) = (e1.as_observable(), e2.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(250));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(201, 240));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(202, 250));
}

#[test]
fn merge_empty_delayed2_left_last() {
    let scheduler = TestScheduler::new();

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);

    let (a, b, s) = (e1.as_observable(), e2.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(250));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(201, 250));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(202, 240));
}

#[test]
fn merge_empty_delayed3_middle_last() {
    let scheduler = TestScheduler::new();

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(245)]);

    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);

    let (a, b, c, s) = (e1.as_observable(), e2.as_observable(), e3.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b, c], s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(250));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(201, 245));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(202, 250));

    assert_subscriptions!(e3.subscriptions(), RT::subscribe(203, 240));
}

#[test]
fn merge_empty_never() {
    let scheduler = TestScheduler::new();

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(245)]);

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b, s) = (e1.as_observable(), n1.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages());

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(201, 245));

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(202, 1000));
}

#[test]
fn merge_never_empty() {
    let scheduler = TestScheduler::new();

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(245)]);

    let (a, b, s) = (n1.as_observable(), e1.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages());

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(201, 1000));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(202, 245));
}

#[test]
fn merge_return_never() {
    let scheduler = TestScheduler::new();

    let r1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(245)]);

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b, s) = (r1.as_observable(), n1.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages(), RT::on_next(210, 2));

    assert_subscriptions!(r1.subscriptions(), RT::subscribe(201, 245));

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(202, 1000));
}

#[test]
fn merge_never_return() {
    let scheduler = TestScheduler::new();

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let r1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(245)]);

    let (a, b, s) = (n1.as_observable(), r1.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages(), RT::on_next(210, 2));

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(201, 1000));

    assert_subscriptions!(r1.subscriptions(), RT::subscribe(202, 245));
}

#[test]
fn merge_error_never() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(245, ex.clone())]);

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b, s) = (e1.as_observable(), n1.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_error(245, ex.clone()));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(201, 245));

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(202, 245));
}

#[test]
fn merge_never_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(245, ex.clone())]);

    let (a, b, s) = (n1.as_observable(), e1.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_error(245, ex.clone()));

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(201, 245));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(202, 245));
}

#[test]
fn merge_empty_return() {
    let scheduler = TestScheduler::new();

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(245)]);

    let r1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let (a, b, s) = (e1.as_observable(), r1.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(250));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(201, 245));

    assert_subscriptions!(r1.subscriptions(), RT::subscribe(202, 250));
}

#[test]
fn merge_return_empty() {
    let scheduler = TestScheduler::new();

    let r1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(245)]);

    let (a, b, s) = (r1.as_observable(), e1.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(250));

    assert_subscriptions!(r1.subscriptions(), RT::subscribe(201, 250));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(202, 245));
}

#[test]
fn merge_lots2() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 4),
        RT::on_next(230, 6),
        RT::on_next(240, 8),
        RT::on_completed(245),
    ]);

    let o2 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(215, 3),
        RT::on_next(225, 5),
        RT::on_next(235, 7),
        RT::on_next(245, 9),
        RT::on_completed(250),
    ]);

    let (a, b, s) = (o1.as_observable(), o2.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b], s));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(215, 3),
        RT::on_next(220, 4),
        RT::on_next(225, 5),
        RT::on_next(230, 6),
        RT::on_next(235, 7),
        RT::on_next(240, 8),
        RT::on_next(245, 9),
        RT::on_completed(250)
    );

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(201, 245));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(202, 250));
}

#[test]
fn merge_lots3() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(225, 5),
        RT::on_next(240, 8),
        RT::on_completed(245),
    ]);

    let o2 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(215, 3),
        RT::on_next(230, 6),
        RT::on_next(245, 9),
        RT::on_completed(250),
    ]);

    let o3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 4), RT::on_next(235, 7), RT::on_completed(240)]);

    let (a, b, c, s) = (o1.as_observable(), o2.as_observable(), o3.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || Observable::merge_all_on(vec![a, b, c], s));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(215, 3),
        RT::on_next(220, 4),
        RT::on_next(225, 5),
        RT::on_next(230, 6),
        RT::on_next(235, 7),
        RT::on_next(240, 8),
        RT::on_next(245, 9),
        RT::on_completed(250)
    );

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(201, 245));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(202, 250));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(203, 240));
}

#[test]
fn merge_lots_more() {
    let mut inputs: Vec<Vec<Recorded<Notification<i32>>>> = Vec::new();

    const N: i64 = 10;
    for i in 0..N {
        let mut lst = vec![RT::on_next(150, 1)];

        // `ushort` arithmetic of upstream: the values stay far below its range.
        let mut start = 301 + i;
        for j in 0..i {
            start += j * 5;
            lst.push(RT::on_next(start, (j + i + 2) as i32));
        }

        lst.push(RT::on_completed(start + N - i));
        inputs.push(lst);
    }

    let inputs_flat: Vec<Recorded<Notification<i32>>> = inputs.iter().flatten().cloned().collect();

    let mut res_on_next: Vec<Recorded<Notification<i32>>> =
        inputs_flat.iter().filter(|n| n.time >= 200).filter(|n| matches!(n.value, Notification::OnNext(_))).cloned().collect();
    res_on_next.sort_by_key(|n| n.time);

    let last_completed = inputs_flat
        .iter()
        .filter(|n| n.time >= 200)
        .filter(|n| matches!(n.value, Notification::OnCompleted))
        .map(|n| n.time)
        .max()
        .expect("a completion");

    let scheduler = TestScheduler::new();

    // Got to create the hot observables now.
    let xss: Vec<Inner> = inputs.iter().map(|lst| scheduler.create_hot_observable(lst.clone()).as_observable()).collect();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::merge_all_on(xss, s));

    let messages = res.messages();
    assert!(res_on_next.len() + 1 == messages.len(), "length");
    for (i, expected) in res_on_next.iter().enumerate() {
        let msg = &messages[i];
        assert!(msg.time == expected.time);
        assert!(matches!(msg.value, Notification::OnNext(_)));
        assert!(msg.value == expected.value);
    }
    let last = &messages[res_on_next.len()];
    assert!(matches!(last.value, Notification::OnCompleted) && last.time == last_completed, "complete");
}

#[test]
fn merge_error_left() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(245, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 3), RT::on_completed(250)]);

    let (a, b, s) = (o1.as_observable(), o2.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || a.merge_on(b, s));

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(215, 3), RT::on_error(245, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(201, 245));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(202, 245));
}

#[test]
fn merge_error_causes_disposal() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(210, ex.clone())]);

    let o1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 1), // should not come
        RT::on_completed(230),
    ]);

    let (a, b, s) = (e1.as_observable(), o1.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || a.merge_on(b, s));

    assert_messages!(res.messages(), RT::on_error::<i32>(210, ex.clone()));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(201, 210));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(202, 210));
}

#[test]
fn merge_observable_of_observable_data() {
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
        RT::on_next(120, 305),
        RT::on_completed(150),
    ]);

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(300, ys1.as_observable()),
        RT::on_next::<Inner>(400, ys2.as_observable()),
        RT::on_next::<Inner>(500, ys3.as_observable()),
        RT::on_completed::<Inner>(600),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.merge_inner());

    assert_messages!(
        res.messages(),
        RT::on_next(310, 101),
        RT::on_next(320, 102),
        RT::on_next(410, 103),
        RT::on_next(410, 201),
        RT::on_next(420, 104),
        RT::on_next(420, 202),
        RT::on_next(430, 203),
        RT::on_next(440, 204),
        RT::on_next(510, 105),
        RT::on_next(510, 301),
        RT::on_next(520, 106),
        RT::on_next(520, 302),
        RT::on_next(530, 303),
        RT::on_next(540, 304),
        RT::on_next(620, 305),
        RT::on_completed(650)
    );

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(300, 530));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(400, 450));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(500, 650));
}

#[test]
fn merge_observable_of_observable_data_non_overlapped() {
    let scheduler = TestScheduler::new();

    let ys1 = scheduler.create_cold_observable(vec![RT::on_next(10, 101), RT::on_next(20, 102), RT::on_completed(230)]);

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
        RT::on_completed(50),
    ]);

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(300, ys1.as_observable()),
        RT::on_next::<Inner>(400, ys2.as_observable()),
        RT::on_next::<Inner>(500, ys3.as_observable()),
        RT::on_completed::<Inner>(600),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.merge_inner());

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
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(300, 530));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(400, 450));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(500, 550));
}

#[test]
fn merge_observable_of_observable_inner_throws() {
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
    let res = scheduler.start(move || source.merge_inner());

    assert_messages!(
        res.messages(),
        RT::on_next(310, 101),
        RT::on_next(320, 102),
        RT::on_next(410, 103),
        RT::on_next(410, 201),
        RT::on_next(420, 104),
        RT::on_next(420, 202),
        RT::on_next(430, 203),
        RT::on_next(440, 204),
        RT::on_error(450, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(300, 450));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(400, 450));

    assert_subscriptions!(ys3.subscriptions());
}

#[test]
fn merge_observable_of_observable_outer_throws() {
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
    let res = scheduler.start(move || source.merge_inner());

    assert_messages!(
        res.messages(),
        RT::on_next(310, 101),
        RT::on_next(320, 102),
        RT::on_next(410, 103),
        RT::on_next(410, 201),
        RT::on_next(420, 104),
        RT::on_next(420, 202),
        RT::on_next(430, 203),
        RT::on_next(440, 204),
        RT::on_error(500, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 500));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(300, 500));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(400, 450));
}

/// The four inner sequences of the `MergeConcat_*` tests; `ys2` completes at `ys2_completed` and
/// `ys3` ends with `ys3_end`.
fn merge_concat_inners(
    scheduler: &TestScheduler,
    ys2_completed: i64,
    ys3_end: Recorded<Notification<i32>>,
) -> [Rc<ferroui_reactive_testing::ColdObservable<i32>>; 4] {
    let ys1 = scheduler.create_cold_observable(vec![RT::on_next(50, 1), RT::on_next(100, 2), RT::on_next(120, 3), RT::on_completed(140)]);

    let ys2 = scheduler.create_cold_observable(vec![RT::on_next(20, 4), RT::on_next(70, 5), RT::on_completed(ys2_completed)]);

    let ys3 = scheduler.create_cold_observable(vec![RT::on_next(10, 6), RT::on_next(90, 7), RT::on_next(110, 8), ys3_end]);

    let ys4 = scheduler.create_cold_observable(vec![RT::on_next(210, 9), RT::on_next(240, 10), RT::on_completed(300)]);

    [ys1, ys2, ys3, ys4]
}

#[test]
fn merge_concat_basic() {
    let scheduler = TestScheduler::new();

    let [ys1, ys2, ys3, ys4] = merge_concat_inners(&scheduler, 200, RT::on_completed(130));

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(210, ys1.as_observable()),
        RT::on_next::<Inner>(260, ys2.as_observable()),
        RT::on_next::<Inner>(270, ys3.as_observable()),
        RT::on_next::<Inner>(320, ys4.as_observable()),
        RT::on_completed::<Inner>(400),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.merge_inner_max(2));

    assert_messages!(
        res.messages(),
        RT::on_next(260, 1),
        RT::on_next(280, 4),
        RT::on_next(310, 2),
        RT::on_next(330, 3),
        RT::on_next(330, 5),
        RT::on_next(360, 6),
        RT::on_next(440, 7),
        RT::on_next(460, 8),
        RT::on_next(670, 9),
        RT::on_next(700, 10),
        RT::on_completed(760)
    );

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(210, 350));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(260, 460));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(350, 480));

    assert_subscriptions!(ys4.subscriptions(), RT::subscribe(460, 760));
}

#[test]
fn merge_concat_basic_long() {
    let scheduler = TestScheduler::new();

    let [ys1, ys2, ys3, ys4] = merge_concat_inners(&scheduler, 300, RT::on_completed(130));

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(210, ys1.as_observable()),
        RT::on_next::<Inner>(260, ys2.as_observable()),
        RT::on_next::<Inner>(270, ys3.as_observable()),
        RT::on_next::<Inner>(320, ys4.as_observable()),
        RT::on_completed::<Inner>(400),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.merge_inner_max(2));

    assert_messages!(
        res.messages(),
        RT::on_next(260, 1),
        RT::on_next(280, 4),
        RT::on_next(310, 2),
        RT::on_next(330, 3),
        RT::on_next(330, 5),
        RT::on_next(360, 6),
        RT::on_next(440, 7),
        RT::on_next(460, 8),
        RT::on_next(690, 9),
        RT::on_next(720, 10),
        RT::on_completed(780)
    );

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(210, 350));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(260, 560));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(350, 480));

    assert_subscriptions!(ys4.subscriptions(), RT::subscribe(480, 780));
}

#[test]
fn merge_concat_basic_wide() {
    let scheduler = TestScheduler::new();

    let [ys1, ys2, ys3, ys4] = merge_concat_inners(&scheduler, 300, RT::on_completed(130));

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(210, ys1.as_observable()),
        RT::on_next::<Inner>(260, ys2.as_observable()),
        RT::on_next::<Inner>(270, ys3.as_observable()),
        RT::on_next::<Inner>(420, ys4.as_observable()),
        RT::on_completed::<Inner>(450),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.merge_inner_max(3));

    assert_messages!(
        res.messages(),
        RT::on_next(260, 1),
        RT::on_next(280, 4),
        RT::on_next(280, 6),
        RT::on_next(310, 2),
        RT::on_next(330, 3),
        RT::on_next(330, 5),
        RT::on_next(360, 7),
        RT::on_next(380, 8),
        RT::on_next(630, 9),
        RT::on_next(660, 10),
        RT::on_completed(720)
    );

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(210, 350));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(260, 560));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(270, 400));

    assert_subscriptions!(ys4.subscriptions(), RT::subscribe(420, 720));
}

#[test]
fn merge_concat_basic_late() {
    let scheduler = TestScheduler::new();

    let [ys1, ys2, ys3, ys4] = merge_concat_inners(&scheduler, 300, RT::on_completed(130));

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(210, ys1.as_observable()),
        RT::on_next::<Inner>(260, ys2.as_observable()),
        RT::on_next::<Inner>(270, ys3.as_observable()),
        RT::on_next::<Inner>(420, ys4.as_observable()),
        RT::on_completed::<Inner>(750),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.merge_inner_max(3));

    assert_messages!(
        res.messages(),
        RT::on_next(260, 1),
        RT::on_next(280, 4),
        RT::on_next(280, 6),
        RT::on_next(310, 2),
        RT::on_next(330, 3),
        RT::on_next(330, 5),
        RT::on_next(360, 7),
        RT::on_next(380, 8),
        RT::on_next(630, 9),
        RT::on_next(660, 10),
        RT::on_completed(750)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 750));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(210, 350));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(260, 560));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(270, 400));

    assert_subscriptions!(ys4.subscriptions(), RT::subscribe(420, 720));
}

#[test]
fn merge_concat_disposed() {
    let scheduler = TestScheduler::new();

    let [ys1, ys2, ys3, ys4] = merge_concat_inners(&scheduler, 200, RT::on_completed(130));

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(210, ys1.as_observable()),
        RT::on_next::<Inner>(260, ys2.as_observable()),
        RT::on_next::<Inner>(270, ys3.as_observable()),
        RT::on_next::<Inner>(320, ys4.as_observable()),
        RT::on_completed::<Inner>(400),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start_disposed(move || source.merge_inner_max(2), 450);

    assert_messages!(
        res.messages(),
        RT::on_next(260, 1),
        RT::on_next(280, 4),
        RT::on_next(310, 2),
        RT::on_next(330, 3),
        RT::on_next(330, 5),
        RT::on_next(360, 6),
        RT::on_next(440, 7)
    );

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(210, 350));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(260, 450));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(350, 450));

    assert_subscriptions!(ys4.subscriptions());
}

#[test]
fn merge_concat_outer_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let [ys1, ys2, ys3, ys4] = merge_concat_inners(&scheduler, 200, RT::on_completed(130));

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(210, ys1.as_observable()),
        RT::on_next::<Inner>(260, ys2.as_observable()),
        RT::on_next::<Inner>(270, ys3.as_observable()),
        RT::on_next::<Inner>(320, ys4.as_observable()),
        RT::on_error::<Inner>(400, ex.clone()),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.merge_inner_max(2));

    assert_messages!(
        res.messages(),
        RT::on_next(260, 1),
        RT::on_next(280, 4),
        RT::on_next(310, 2),
        RT::on_next(330, 3),
        RT::on_next(330, 5),
        RT::on_next(360, 6),
        RT::on_error(400, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(210, 350));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(260, 400));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(350, 400));

    assert_subscriptions!(ys4.subscriptions());
}

#[test]
fn merge_concat_inner_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let [ys1, ys2, ys3, ys4] = merge_concat_inners(&scheduler, 200, RT::on_error(140, ex.clone()));

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next::<Inner>(210, ys1.as_observable()),
        RT::on_next::<Inner>(260, ys2.as_observable()),
        RT::on_next::<Inner>(270, ys3.as_observable()),
        RT::on_next::<Inner>(320, ys4.as_observable()),
        RT::on_completed::<Inner>(400),
    ]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.merge_inner_max(2));

    assert_messages!(
        res.messages(),
        RT::on_next(260, 1),
        RT::on_next(280, 4),
        RT::on_next(310, 2),
        RT::on_next(330, 3),
        RT::on_next(330, 5),
        RT::on_next(360, 6),
        RT::on_next(440, 7),
        RT::on_next(460, 8),
        RT::on_error(490, ex.clone())
    );

    // BREAKING CHANGE v2 > v1.x -> More aggressive disposal behavior
    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_subscriptions!(ys1.subscriptions(), RT::subscribe(210, 350));

    assert_subscriptions!(ys2.subscriptions(), RT::subscribe(260, 460));

    assert_subscriptions!(ys3.subscriptions(), RT::subscribe(350, 490));

    assert_subscriptions!(ys4.subscriptions(), RT::subscribe(460, 490));
}

// Not ported: Merge_ArgumentChecking - argument validation
// Not ported: Merge_DefaultScheduler - needs operators ToList, Single (phase 3, later)
// Not ported: Merge_Binary_DefaultScheduler - needs operator ToEnumerable (phase 3, later)
// Not ported: Merge_Params_DefaultScheduler - needs operator ToEnumerable (phase 3, later)
// Not ported: Merge_IEnumerableOfIObservable_DefaultScheduler - needs operator ToEnumerable (phase 3, later)
// Not ported: MergeConcat_ArgumentChecking - argument validation
// Not ported: MergeConcat_Enumerable_Scheduler - needs operators Range, SequenceEqual, First (phase 3, later)
// Not ported: MergeConcat_Enumerable - needs operators Range, SequenceEqual, First (phase 3, later); real threads (DefaultScheduler)
// Not ported: MergeConcat_Default - needs operators Range, SequenceEqual, First (phase 3, later)
// Not ported: Merge_Task - needs async/await (phase 4)
// Not ported: Merge_TaskWithCompletionSource_RanToCompletion_Async - needs async/await (phase 4)
// Not ported: Merge_TaskWithCompletionSource_RanToCompletion_Sync - needs async/await (phase 4)
// Not ported: Merge_TaskWithCompletionSource_Faulted_Async - needs async/await (phase 4)
// Not ported: Merge_TaskWithCompletionSource_Faulted_Sync - needs async/await (phase 4)
// Not ported: Merge_TaskWithCompletionSource_Canceled_Async - needs async/await (phase 4)
// Not ported: Merge_TaskWithCompletionSource_Canceled_Sync - needs async/await (phase 4)
// Not ported: Merge_TaskWithCompletionSource_InnerCompleteBeforeOuter - needs async/await (phase 4)
// Not ported: Merge_TaskWithCompletionSource_OuterCompleteBeforeInner - needs async/await (phase 4)
// Not ported: Merge_Task_OnError - needs async/await (phase 4)
