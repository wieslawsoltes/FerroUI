//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DoWhileTest.cs` of Rx.NET.
//! Adapted: `Observable.DoWhile(source, condition)` is the method `do_while`; a condition that
//! throws is the `try_do_while` form returning the error.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ColdObservable, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

fn source(scheduler: &TestScheduler) -> Rc<ColdObservable<i32>> {
    scheduler.create_cold_observable(vec![
        RT::on_next(50, 1),
        RT::on_next(100, 2),
        RT::on_next(150, 3),
        RT::on_next(200, 4),
        RT::on_completed(250),
    ])
}

#[test]
fn do_while_always_false() {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler);

    let x = xs.as_observable();
    let results = scheduler.start(move || x.do_while(|| false));

    assert_messages!(
        results.messages(),
        RT::on_next(250, 1),
        RT::on_next(300, 2),
        RT::on_next(350, 3),
        RT::on_next(400, 4),
        RT::on_completed(450)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn do_while_always_true() {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler);

    let x = xs.as_observable();
    let results = scheduler.start(move || x.do_while(|| true));

    assert_messages!(
        results.messages(),
        RT::on_next(250, 1),
        RT::on_next(300, 2),
        RT::on_next(350, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 1),
        RT::on_next(550, 2),
        RT::on_next(600, 3),
        RT::on_next(650, 4),
        RT::on_next(750, 1),
        RT::on_next(800, 2),
        RT::on_next(850, 3),
        RT::on_next(900, 4)
    );

    assert_subscriptions!(
        xs.subscriptions(),
        RT::subscribe(200, 450),
        RT::subscribe(450, 700),
        RT::subscribe(700, 950),
        RT::subscribe(950, 1000)
    );
}

#[test]
fn do_while_always_true_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_error::<i32>(50, ex.clone())]);

    let x = xs.as_observable();
    let results = scheduler.start(move || x.do_while(|| true));

    assert_messages!(results.messages(), RT::on_error(250, ex));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do_while_always_true_infinite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(50, 1)]);

    let x = xs.as_observable();
    let results = scheduler.start(move || x.do_while(|| true));

    assert_messages!(results.messages(), RT::on_next(250, 1));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn do_while_sometimes_true() {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler);

    let n = Rc::new(Cell::new(0));

    let x = xs.as_observable();
    let results = scheduler.start(move || {
        x.do_while(move || {
            n.set(n.get() + 1);
            n.get() < 3
        })
    });

    assert_messages!(
        results.messages(),
        RT::on_next(250, 1),
        RT::on_next(300, 2),
        RT::on_next(350, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 1),
        RT::on_next(550, 2),
        RT::on_next(600, 3),
        RT::on_next(650, 4),
        RT::on_next(750, 1),
        RT::on_next(800, 2),
        RT::on_next(850, 3),
        RT::on_next(900, 4),
        RT::on_completed(950)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450), RT::subscribe(450, 700), RT::subscribe(700, 950));
}

#[test]
fn do_while_sometimes_throws() {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler);

    let n = Rc::new(Cell::new(0));

    let ex = new_error("ex");

    let (x, e) = (xs.as_observable(), ex.clone());
    let results = scheduler.start(move || {
        x.try_do_while(move || {
            n.set(n.get() + 1);
            if n.get() < 3 {
                Ok(true)
            } else {
                Err(e.clone())
            }
        })
    });

    assert_messages!(
        results.messages(),
        RT::on_next(250, 1),
        RT::on_next(300, 2),
        RT::on_next(350, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 1),
        RT::on_next(550, 2),
        RT::on_next(600, 3),
        RT::on_next(650, 4),
        RT::on_next(750, 1),
        RT::on_next(800, 2),
        RT::on_next(850, 3),
        RT::on_next(900, 4),
        RT::on_error(950, ex)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450), RT::subscribe(450, 700), RT::subscribe(700, 950));
}

// Not ported: DoWhile_ArgumentChecking - argument validation
