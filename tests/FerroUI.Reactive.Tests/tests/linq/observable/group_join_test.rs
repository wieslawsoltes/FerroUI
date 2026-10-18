//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/GroupJoinTest.cs` of Rx.NET.
//! Adapted: the anonymous values are `TimeInterval<i32>` and `TimeInterval<String>`; the duration
//! selectors take the value by reference; `Merge()` of the windows is `merge_inner`; selectors that
//! throw are the `try_group_join` form returning the error; the expectations are upstream's
//! `!NO_PERF` ones (the more aggressive disposal of its version 2, and the duration selector
//! evaluated before the result selector); `NewTimer` and `AssertDurations` are ported below.

use ferroui_reactive::{IObservable, Notification, Observable, ObservableError, ObservableExt, TimeInterval};
use ferroui_reactive_testing::{
    assert_messages, assert_subscriptions, duration_to_ticks, new_error, ticks_to_duration, ColdObservable, HotObservable, ReactiveTest as RT, Recorded,
    TestScheduler,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

type TI32 = TimeInterval<i32>;
type TIS = TimeInterval<String>;
type Timers = Rc<RefCell<Vec<Rc<ColdObservable<i64>>>>>;

/// A value of the left sequence: `OnNext(time, new TimeInterval<int>(value, TimeSpan.FromTicks(ticks)))`.
fn x(time: i64, value: i32, ticks: i64) -> Recorded<Notification<TI32>> {
    RT::on_next(time, TimeInterval::new(value, ticks_to_duration(ticks)))
}

/// A value of the right sequence: `OnNext(time, new TimeInterval<string>(value, TimeSpan.FromTicks(ticks)))`.
fn y(time: i64, value: &str, ticks: i64) -> Recorded<Notification<TIS>> {
    RT::on_next(time, TimeInterval::new(value.to_owned(), ticks_to_duration(ticks)))
}

/// A result: `OnNext(time, value)`.
fn n(time: i64, value: &str) -> Recorded<Notification<String>> {
    RT::on_next(time, value.to_owned())
}

fn timers() -> Timers {
    Rc::new(RefCell::new(Vec::new()))
}

/// `NewTimer`: a cold sequence that notifies at `t` and completes, recorded in `l`.
fn new_timer(l: &Timers, t: Duration, scheduler: &TestScheduler) -> Rc<dyn IObservable<i64>> {
    let ticks = duration_to_ticks(t);
    let timer = scheduler.create_cold_observable(vec![RT::on_next(ticks, 0i64), RT::on_completed(ticks)]);
    l.borrow_mut().push(timer.clone());
    timer.as_observable()
}

/// `AssertDurations`: the duration of each value was subscribed to from the value to the end of
/// its interval, or to `last_end`.
fn assert_durations<T: Clone + 'static>(xs: &Rc<HotObservable<TimeInterval<T>>>, xsd: &Timers, last_end: i64) {
    let messages = xs.messages();
    let xsd = xsd.borrow();
    assert_eq!(messages.iter().filter(|m| matches!(m.value, Notification::OnNext(_)) && m.time <= last_end).count(), xsd.len());

    for (message, duration) in messages.iter().zip(xsd.iter()) {
        let start = message.time;
        let interval = message.value.value().expect("a value of the sequence").interval;
        let end = (start + duration_to_ticks(interval)).min(last_end);
        assert_subscriptions!(duration.subscriptions(), RT::subscribe(start, end));
    }
}

/// `(x, yy) => yy.Select(y => x.Value + y.Value)`.
fn window_sums(x: TI32, yy: Rc<dyn IObservable<TIS>>) -> Rc<dyn IObservable<String>> {
    let value = x.value;
    yy.select(move |y: TIS| format!("{}{}", value, y.value))
}

/// `xs.GroupJoin(ys, x => NewTimer(xsd, x.Interval, scheduler), y => NewTimer(ysd, y.Interval, scheduler), (x, yy) => yy.Select(y => x.Value + y.Value)).Merge()`.
fn group_join_timers(
    scheduler: &TestScheduler,
    xs: &Rc<HotObservable<TI32>>,
    ys: &Rc<HotObservable<TIS>>,
    xsd: &Timers,
    ysd: &Timers,
) -> impl FnOnce() -> Rc<dyn IObservable<String>> + 'static {
    let (left, right) = (xs.as_observable(), ys.as_observable());
    let (xsd, ysd, s1, s2) = (xsd.clone(), ysd.clone(), scheduler.clone(), scheduler.clone());
    move || {
        left.group_join(right, move |x: &TI32| new_timer(&xsd, x.interval, &s1), move |y: &TIS| new_timer(&ysd, y.interval, &s2), window_sums)
            .merge_inner::<String>()
    }
}

/// The left sequence of ten values whose eighth lasts `seventh_ticks`, ending with `end`.
fn xs_ten(scheduler: &TestScheduler, seventh_ticks: i64, end: Recorded<Notification<TI32>>) -> Rc<HotObservable<TI32>> {
    scheduler.create_hot_observable(vec![
        x(210, 0, 10),
        x(219, 1, 5),
        x(240, 2, 10),
        x(300, 3, 100),
        x(310, 4, 80),
        x(500, 5, 90),
        x(700, 6, 25),
        x(710, 7, seventh_ticks),
        x(720, 8, 100),
        x(830, 9, 10),
        end,
    ])
}

/// The left sequence of nine values whose eighth lasts 200 ticks, ending with `end`.
fn xs_nine(scheduler: &TestScheduler, end: Recorded<Notification<TI32>>) -> Rc<HotObservable<TI32>> {
    scheduler.create_hot_observable(vec![
        x(210, 0, 10),
        x(219, 1, 5),
        x(240, 2, 10),
        x(300, 3, 100),
        x(310, 4, 80),
        x(500, 5, 90),
        x(700, 6, 25),
        x(710, 7, 200),
        x(720, 8, 100),
        end,
    ])
}

/// The right sequence of most tests, ending with `end`.
fn ys_standard(scheduler: &TestScheduler, end: Recorded<Notification<TIS>>) -> Rc<HotObservable<TIS>> {
    scheduler.create_hot_observable(vec![
        y(215, "hat", 20),
        y(217, "bat", 1),
        y(290, "wag", 200),
        y(300, "pig", 10),
        y(305, "cup", 50),
        y(600, "yak", 90),
        y(702, "tin", 20),
        y(712, "man", 10),
        y(722, "rat", 200),
        y(732, "wig", 5),
        end,
    ])
}

/// The right sequence whose "rat" lasts 20 ticks, ending at 900.
fn ys_short_rat(scheduler: &TestScheduler) -> Rc<HotObservable<TIS>> {
    scheduler.create_hot_observable(vec![
        y(215, "hat", 20),
        y(217, "bat", 1),
        y(290, "wag", 200),
        y(300, "pig", 10),
        y(305, "cup", 50),
        y(600, "yak", 90),
        y(702, "tin", 20),
        y(712, "man", 10),
        y(722, "rat", 20),
        y(732, "wig", 5),
        RT::on_completed(900),
    ])
}

#[test]
fn group_join_op_normal_i() {
    let scheduler = TestScheduler::new();

    let xs = xs_ten(&scheduler, 280, RT::on_completed(900));

    let ys = ys_standard(&scheduler, RT::on_completed(800));

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd));

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man"),
        n(720, "8tin"),
        n(720, "8man"),
        n(722, "6rat"),
        n(722, "7rat"),
        n(722, "8rat"),
        n(732, "7wig"),
        n(732, "8wig"),
        n(830, "9rat"),
        RT::on_completed(990)
    );

    assert_durations(&xs, &xsd, 990);
    assert_durations(&ys, &ysd, 990);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 900));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 800));
}

#[test]
fn group_join_op_normal_ii() {
    let scheduler = TestScheduler::new();

    let xs = xs_nine(&scheduler, RT::on_completed(721));

    let ys = ys_standard(&scheduler, RT::on_completed(990));

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd));

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man"),
        n(720, "8tin"),
        n(720, "8man"),
        n(722, "6rat"),
        n(722, "7rat"),
        n(722, "8rat"),
        n(732, "7wig"),
        n(732, "8wig"),
        RT::on_completed(910)
    );

    assert_durations(&xs, &xsd, 910);
    assert_durations(&ys, &ysd, 910);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 721));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 910));
}

#[test]
fn group_join_op_normal_iii() {
    let scheduler = TestScheduler::new();

    let xs = xs_ten(&scheduler, 280, RT::on_completed(900));

    let ys = ys_standard(&scheduler, RT::on_completed(800));

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start({
        let (left, right) = (xs.as_observable(), ys.as_observable());
        let (xsd, ysd, s1, s2) = (xsd.clone(), ysd.clone(), scheduler.clone(), scheduler.clone());
        move || {
            left.group_join(
                right,
                move |x: &TI32| new_timer(&xsd, x.interval, &s1).where_(|_| false),
                move |y: &TIS| new_timer(&ysd, y.interval, &s2).where_(|_| false),
                window_sums,
            )
            .merge_inner::<String>()
        }
    });

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man"),
        n(720, "8tin"),
        n(720, "8man"),
        n(722, "6rat"),
        n(722, "7rat"),
        n(722, "8rat"),
        n(732, "7wig"),
        n(732, "8wig"),
        n(830, "9rat"),
        RT::on_completed(990)
    );

    assert_durations(&xs, &xsd, 990);
    assert_durations(&ys, &ysd, 990);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 900));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 800));
}

#[test]
fn group_join_op_normal_iv() {
    let scheduler = TestScheduler::new();

    let xs = xs_nine(&scheduler, RT::on_completed(990));

    let ys = ys_standard(&scheduler, RT::on_completed(980));

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd));

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man"),
        n(720, "8tin"),
        n(720, "8man"),
        n(722, "6rat"),
        n(722, "7rat"),
        n(722, "8rat"),
        n(732, "7wig"),
        n(732, "8wig"),
        RT::on_completed(990)
    );

    assert_durations(&xs, &xsd, 990);
    assert_durations(&ys, &ysd, 990);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 990));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 980));
}

#[test]
fn group_join_op_normal_v() {
    let scheduler = TestScheduler::new();

    let xs = xs_nine(&scheduler, RT::on_completed(990));

    let ys = ys_standard(&scheduler, RT::on_completed(900));

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd));

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man"),
        n(720, "8tin"),
        n(720, "8man"),
        n(722, "6rat"),
        n(722, "7rat"),
        n(722, "8rat"),
        n(732, "7wig"),
        n(732, "8wig"),
        RT::on_completed(990)
    );

    assert_durations(&xs, &xsd, 990);
    assert_durations(&ys, &ysd, 990);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 990));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 900));
}

#[test]
fn group_join_op_normal_vi() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        x(210, 0, 10),
        x(219, 1, 5),
        x(240, 2, 10),
        x(300, 3, 100),
        x(310, 4, 80),
        x(500, 5, 90),
        x(700, 6, 25),
        x(710, 7, 30),
        x(720, 8, 200),
        x(830, 9, 10),
        RT::on_completed(850),
    ]);

    let ys = ys_short_rat(&scheduler);

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd));

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man"),
        n(720, "8tin"),
        n(720, "8man"),
        n(722, "6rat"),
        n(722, "7rat"),
        n(722, "8rat"),
        n(732, "7wig"),
        n(732, "8wig"),
        RT::on_completed(920)
    );

    assert_durations(&xs, &xsd, 920);
    assert_durations(&ys, &ysd, 920);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 850));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 900));
}

#[test]
fn group_join_op_normal_vii() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<TI32>(210)]);

    let ys = ys_short_rat(&scheduler);

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd));

    assert_messages!(res.messages(), RT::on_completed::<String>(210));

    assert_durations(&xs, &xsd, 210);
    assert_durations(&ys, &ysd, 210);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn group_join_op_normal_viii() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![x(210, 0, 200)]);

    let ys = scheduler.create_hot_observable(vec![y(220, "hat", 100), RT::on_completed(230)]);

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd));

    assert_messages!(res.messages(), n(220, "0hat"));

    assert_durations(&xs, &xsd, 1000);
    assert_durations(&ys, &ysd, 1000);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn group_join_op_normal_ix() {
    let scheduler = TestScheduler::new();

    let xs = xs_ten(&scheduler, 300, RT::on_completed(900));

    let ys = ys_standard(&scheduler, RT::on_completed(800));

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start_disposed(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd), 713);

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man")
    );

    assert_durations(&xs, &xsd, 713);
    assert_durations(&ys, &ysd, 713);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 713));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 713));
}

#[test]
fn group_join_op_error_i() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![x(210, 0, 10), x(219, 1, 5), x(240, 2, 10), x(300, 3, 100), RT::on_error(310, ex.clone())]);

    let ys = ys_standard(&scheduler, RT::on_completed(800));

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd));

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        RT::on_error(310, ex.clone())
    );

    assert_durations(&xs, &xsd, 310);
    assert_durations(&ys, &ysd, 310);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 310));
}

#[test]
fn group_join_op_error_ii() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = xs_ten(&scheduler, 300, RT::on_completed(900));

    let ys = scheduler.create_hot_observable(vec![
        y(215, "hat", 20),
        y(217, "bat", 1),
        y(290, "wag", 200),
        y(300, "pig", 10),
        y(305, "cup", 50),
        y(600, "yak", 90),
        y(702, "tin", 20),
        y(712, "man", 10),
        RT::on_error(722, ex.clone()),
    ]);

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(group_join_timers(&scheduler, &xs, &ys, &xsd, &ysd));

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man"),
        n(720, "8tin"),
        n(720, "8man"),
        RT::on_error(722, ex.clone())
    );

    assert_durations(&xs, &xsd, 722);
    assert_durations(&ys, &ysd, 722);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 722));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 722));
}

#[test]
fn group_join_op_error_iii() {
    let scheduler = TestScheduler::new();

    let xs = xs_ten(&scheduler, 300, RT::on_completed(900));

    let ys = ys_standard(&scheduler, RT::on_completed(800));

    let ex = new_error("ex");

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start({
        let (left, right) = (xs.as_observable(), ys.as_observable());
        let (xsd, ysd, s1, s2, ex) = (xsd.clone(), ysd.clone(), scheduler.clone(), scheduler.clone(), ex.clone());
        move || {
            left.group_join(
                right,
                move |x: &TI32| {
                    new_timer(&xsd, x.interval, &s1).select_many_to(if x.value == 6 { Observable::throw::<i64>(ex.clone()) } else { Observable::empty::<i64>() })
                },
                move |y: &TIS| new_timer(&ysd, y.interval, &s2),
                window_sums,
            )
            .merge_inner::<String>()
        }
    });

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man"),
        n(720, "8tin"),
        n(720, "8man"),
        n(722, "6rat"),
        n(722, "7rat"),
        n(722, "8rat"),
        RT::on_error(725, ex.clone())
    );

    assert_durations(&xs, &xsd, 725);
    assert_durations(&ys, &ysd, 725);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 725));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 725));
}

#[test]
fn group_join_op_error_iv() {
    let scheduler = TestScheduler::new();

    let xs = xs_ten(&scheduler, 300, RT::on_completed(900));

    let ys = scheduler.create_hot_observable(vec![
        y(215, "hat", 20),
        y(217, "bat", 1),
        y(290, "wag", 200),
        y(300, "pig", 10),
        y(305, "cup", 50),
        y(600, "yak", 90),
        y(702, "tin", 19),
        y(712, "man", 10),
        y(722, "rat", 200),
        y(732, "wig", 5),
        RT::on_completed(800),
    ]);

    let ex = new_error("ex");

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start({
        let (left, right) = (xs.as_observable(), ys.as_observable());
        let (xsd, ysd, s1, s2, ex) = (xsd.clone(), ysd.clone(), scheduler.clone(), scheduler.clone(), ex.clone());
        move || {
            left.group_join(
                right,
                move |x: &TI32| new_timer(&xsd, x.interval, &s1),
                move |y: &TIS| {
                    new_timer(&ysd, y.interval, &s2).select_many_to(if y.value == "tin" { Observable::throw::<i64>(ex.clone()) } else { Observable::empty::<i64>() })
                },
                window_sums,
            )
            .merge_inner::<String>()
        }
    });

    assert_messages!(
        res.messages(),
        n(215, "0hat"),
        n(217, "0bat"),
        n(219, "1hat"),
        n(300, "3wag"),
        n(300, "3pig"),
        n(305, "3cup"),
        n(310, "4wag"),
        n(310, "4pig"),
        n(310, "4cup"),
        n(702, "6tin"),
        n(710, "7tin"),
        n(712, "6man"),
        n(712, "7man"),
        n(720, "8tin"),
        n(720, "8man"),
        RT::on_error(721, ex.clone())
    );

    assert_durations(&xs, &xsd, 721);
    assert_durations(&ys, &ysd, 721);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 721));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 721));
}

#[test]
fn group_join_op_error_v() {
    let scheduler = TestScheduler::new();

    let xs = xs_ten(&scheduler, 300, RT::on_completed(900));

    let ys = ys_standard(&scheduler, RT::on_completed(800));

    let ex = new_error("ex");

    let ysd = timers();

    let res = scheduler.start({
        let (left, right) = (xs.as_observable(), ys.as_observable());
        let (ysd, s, ex) = (ysd.clone(), scheduler.clone(), ex.clone());
        move || {
            left.try_group_join(
                right,
                move |x: &TI32| if x.value >= 0 { Err(ex.clone()) } else { Ok(Observable::empty::<i64>()) },
                move |y: &TIS| Ok(new_timer(&ysd, y.interval, &s)),
                |x, yy| Ok(window_sums(x, yy)),
            )
            .merge_inner::<String>()
        }
    });

    assert_messages!(res.messages(), RT::on_error::<String>(210, ex.clone()));

    assert_durations(&ys, &ysd, 210);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn group_join_op_error_vi() {
    let scheduler = TestScheduler::new();

    let xs = xs_ten(&scheduler, 300, RT::on_completed(900));

    let ys = ys_standard(&scheduler, RT::on_completed(800));

    let ex = new_error("ex");

    let xsd = timers();

    let res = scheduler.start({
        let (left, right) = (xs.as_observable(), ys.as_observable());
        let (xsd, s, ex) = (xsd.clone(), scheduler.clone(), ex.clone());
        move || {
            left.try_group_join(
                right,
                move |x: &TI32| Ok(new_timer(&xsd, x.interval, &s)),
                // `y => { if (y.Value.Length >= 0) { throw ex; } return Observable.Empty<long>(); }`: the
                // condition always holds.
                move |_: &TIS| -> Result<Rc<dyn IObservable<i64>>, ObservableError> { Err(ex.clone()) },
                |x, yy| Ok(window_sums(x, yy)),
            )
            .merge_inner::<String>()
        }
    });

    assert_messages!(res.messages(), RT::on_error::<String>(215, ex.clone()));

    assert_durations(&xs, &xsd, 215);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn group_join_op_error_vii() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        x(215, 0, 10),
        x(219, 1, 5),
        x(240, 2, 10),
        x(300, 3, 100),
        x(310, 4, 80),
        x(500, 5, 90),
        x(700, 6, 25),
        x(710, 7, 300),
        x(720, 8, 100),
        x(830, 9, 10),
        RT::on_completed(900),
    ]);

    let ys = scheduler.create_hot_observable(vec![
        y(210, "hat", 20),
        y(217, "bat", 1),
        y(290, "wag", 200),
        y(300, "pig", 10),
        y(305, "cup", 50),
        y(600, "yak", 90),
        y(702, "tin", 20),
        y(712, "man", 10),
        y(722, "rat", 200),
        y(732, "wig", 5),
        RT::on_completed(800),
    ]);

    let ex = new_error("ex");

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(try_group_join_result_throws(&scheduler, &xs, &ys, &xsd, &ysd, &ex));

    assert_messages!(res.messages(), RT::on_error::<String>(215, ex.clone()));

    assert_durations(&xs, &xsd, 215);
    assert_durations(&ys, &ysd, 215);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn group_join_op_error_viii() {
    let scheduler = TestScheduler::new();

    let xs = xs_ten(&scheduler, 300, RT::on_completed(900));

    let ys = ys_standard(&scheduler, RT::on_completed(800));

    let ex = new_error("ex");

    let (xsd, ysd) = (timers(), timers());

    let res = scheduler.start(try_group_join_result_throws(&scheduler, &xs, &ys, &xsd, &ysd, &ex));

    assert_messages!(res.messages(), RT::on_error::<String>(210, ex.clone()));

    assert_durations(&xs, &xsd, 210);
    assert_durations(&ys, &ysd, 210);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 210));
}

/// The group join of `GroupJoinOp_Error_VII` and `GroupJoinOp_Error_VIII`: timers as durations, and
/// a result selector that throws (`(x, yy) => { if (x.Value >= 0) { throw ex; } return
/// yy.Select(y => x.Value + y.Value); }`), merged.
fn try_group_join_result_throws(
    scheduler: &TestScheduler,
    xs: &Rc<HotObservable<TI32>>,
    ys: &Rc<HotObservable<TIS>>,
    xsd: &Timers,
    ysd: &Timers,
    ex: &ObservableError,
) -> impl FnOnce() -> Rc<dyn IObservable<String>> + 'static {
    let (left, right) = (xs.as_observable(), ys.as_observable());
    let (xsd, ysd, s1, s2, ex) = (xsd.clone(), ysd.clone(), scheduler.clone(), scheduler.clone(), ex.clone());
    move || {
        left.try_group_join(
            right,
            move |x: &TI32| Ok(new_timer(&xsd, x.interval, &s1)),
            move |y: &TIS| Ok(new_timer(&ysd, y.interval, &s2)),
            move |x: TI32, yy: Rc<dyn IObservable<TIS>>| if x.value >= 0 { Err(ex.clone()) } else { Ok(window_sums(x, yy)) },
        )
        .merge_inner::<String>()
    }
}

// Not ported: GroupJoinOp_ArgumentChecking - argument validation
