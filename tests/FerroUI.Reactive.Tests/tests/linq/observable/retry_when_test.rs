//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RetryWhenTest.cs` of Rx.NET.
//! Adapted: a handler that throws is the `try_` form returning the error.

use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn retry_when_observable_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_completed(250)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().retry_when(|v| v)
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_completed(450));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn retry_when_observable_handler_completes() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_error(250, ex)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().retry_when(|v| v.take(1).skip(1))
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_completed(450));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn retry_when_observable_handler_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let res = scheduler.start({
        let ex = ex.clone();
        move || Observable::return_(1).try_retry_when(move |_| Err::<Rc<dyn IObservable<i32>>, _>(ex.clone()))
    });

    assert_messages!(res.messages(), RT::on_error(200, ex.clone()));
}

#[test]
fn retry_when_observable_handler_errors() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");
    let ex2 = new_error("ex2");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_error(250, ex)]);

    let res = scheduler.start({
        let (xs, ex2) = (xs.clone(), ex2.clone());
        move || {
            xs.as_observable().retry_when(move |v| {
                let ex2 = ex2.clone();
                v.select_many(move |_w| Observable::throw::<i32>(ex2.clone()))
            })
        }
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_error(450, ex2.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn retry_when_observable_retry_count_basic() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(5, 1), RT::on_next(10, 2), RT::on_next(15, 3), RT::on_error(20, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || {
            xs.as_observable().retry_when(|v| {
                let count = Rc::new(Cell::new(0));
                v.select_many(move |w| {
                    count.set(count.get() + 1);
                    let c = count.get();
                    if c == 3 {
                        return Observable::throw::<i32>(w);
                    }
                    Observable::return_(1)
                })
            })
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(205, 1),
        RT::on_next(210, 2),
        RT::on_next(215, 3),
        RT::on_next(225, 1),
        RT::on_next(230, 2),
        RT::on_next(235, 3),
        RT::on_next(245, 1),
        RT::on_next(250, 2),
        RT::on_next(255, 3),
        RT::on_error(260, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220), RT::subscribe(220, 240), RT::subscribe(240, 260));
}

#[test]
fn retry_when_observable_retry_count_delayed() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(5, 1), RT::on_next(10, 2), RT::on_next(15, 3), RT::on_error(20, ex.clone())]);

    let res = scheduler.start({
        let (xs, sch) = (xs.clone(), scheduler.scheduler());
        move || {
            xs.as_observable().retry_when(move |v| {
                let count = Rc::new(Cell::new(0));
                let sch = sch.clone();
                v.select_many(move |w| {
                    count.set(count.get() + 1);
                    let c = count.get();
                    if c == 3 {
                        return Observable::throw::<i32>(w);
                    }
                    Observable::return_(1).delay_on(ticks_to_duration(c * 100), sch.clone())
                })
            })
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(205, 1),
        RT::on_next(210, 2),
        RT::on_next(215, 3),
        RT::on_next(325, 1),
        RT::on_next(330, 2),
        RT::on_next(335, 3),
        RT::on_next(545, 1),
        RT::on_next(550, 2),
        RT::on_next(555, 3),
        RT::on_error(560, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220), RT::subscribe(320, 340), RT::subscribe(540, 560));
}

// Not ported: RetryWhen_Observable_ArgumentChecking - argument validation
