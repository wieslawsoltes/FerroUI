//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/WhileTest.cs` of Rx.NET.
//! Adapted: `Observable.While` is `Observable::while_`; a condition that throws is the `try_while`
//! form returning the error. The general loop tests (`LoopTest1`, `LoopTest2`, compiled upstream
//! with `HAS_STACKTRACE`) check their values with a subscription, which delivers them before it
//! returns (the sequences are synchronous; upstream's `ForEach` blocks until the end); their check
//! of the depth of the stack is not ported (see the end of the file).

use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn while_always_false() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![
        RT::on_next(50, 1),
        RT::on_next(100, 2),
        RT::on_next(150, 3),
        RT::on_next(200, 4),
        RT::on_completed(250),
    ]);

    let x = xs.as_observable();
    let results = scheduler.start(move || Observable::while_(|| false, x));

    assert_messages!(results.messages(), RT::on_completed::<i32>(200));

    assert_subscriptions!(xs.subscriptions());
}

#[test]
fn while_always_true() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![
        RT::on_next(50, 1),
        RT::on_next(100, 2),
        RT::on_next(150, 3),
        RT::on_next(200, 4),
        RT::on_completed(250),
    ]);

    let x = xs.as_observable();
    let results = scheduler.start(move || Observable::while_(|| true, x));

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
fn while_always_true_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_error::<i32>(50, ex.clone())]);

    let x = xs.as_observable();
    let results = scheduler.start(move || Observable::while_(|| true, x));

    assert_messages!(results.messages(), RT::on_error(250, ex));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn while_always_true_infinite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(50, 1)]);

    let x = xs.as_observable();
    let results = scheduler.start(move || Observable::while_(|| true, x));

    assert_messages!(results.messages(), RT::on_next(250, 1));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn while_sometimes_true() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![
        RT::on_next(50, 1),
        RT::on_next(100, 2),
        RT::on_next(150, 3),
        RT::on_next(200, 4),
        RT::on_completed(250),
    ]);

    let n = Rc::new(Cell::new(0));

    let x = xs.as_observable();
    let results = scheduler.start(move || {
        Observable::while_(
            move || {
                n.set(n.get() + 1);
                n.get() < 3
            },
            x,
        )
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
        RT::on_completed(700)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450), RT::subscribe(450, 700));
}

#[test]
fn while_sometimes_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![
        RT::on_next(50, 1),
        RT::on_next(100, 2),
        RT::on_next(150, 3),
        RT::on_next(200, 4),
        RT::on_completed(250),
    ]);

    let n = Rc::new(Cell::new(0));

    let ex = new_error("ex");

    let (x, e) = (xs.as_observable(), ex.clone());
    let results = scheduler.start(move || {
        Observable::try_while(
            move || {
                n.set(n.get() + 1);
                if n.get() < 3 {
                    Ok(true)
                } else {
                    Err(e.clone())
                }
            },
            x,
        )
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
        RT::on_error(700, ex)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450), RT::subscribe(450, 700));
}

#[test]
fn loop_test1() {
    let loop_ = Observable::defer(|| {
        let n = Rc::new(Cell::new(0));
        let inner = n.clone();
        Observable::while_(
            move || {
                let current = n.get();
                n.set(current + 1);
                current < 5
            },
            Observable::defer(move || {
                let count = inner.get();
                Observable::for_(0..count, Observable::return_)
            }),
        )
    });

    let res = Rc::new(RefCell::new(Vec::new()));
    let r = res.clone();
    let _d = loop_.subscribe_next(move |x| r.borrow_mut().push(x));

    assert_eq!(vec![0, 0, 1, 0, 1, 2, 0, 1, 2, 3, 0, 1, 2, 3, 4], *res.borrow());
}

#[test]
fn loop_test2() {
    let n = Rc::new(Cell::new(0));

    let loop_: Rc<RefCell<Option<Rc<dyn IObservable<i32>>>>> = Rc::new(RefCell::new(None));
    let inner = loop_.clone();
    let condition = n.clone();
    *loop_.borrow_mut() = Some(Observable::while_(
        move || {
            let current = condition.get();
            condition.set(current + 1);
            current < 10
        },
        Observable::concat_all(vec![
            Observable::return_(42),
            Observable::defer(move || inner.borrow().clone().expect("the loop is set")),
        ]),
    ));

    let res = Rc::new(RefCell::new(Vec::new()));
    let r = res.clone();
    let looped = loop_.borrow().clone().expect("the loop is set");
    let _d = looped.subscribe_next(move |x| r.borrow_mut().push(x));

    assert_eq!(vec![42; 10], *res.borrow());

    // The loop refers to itself; release it.
    *loop_.borrow_mut() = None;
}

// Not ported: While_ArgumentChecking - argument validation
// Not ported (in part): LoopTest1, LoopTest2 - their check that every value is notified at the same depth of the stack (`StackTrace.FrameCount`); the port has no stack traces, and its loops over sequences that end at once do not flatten nested loops (DEVIATIONS.md, `TailRecursiveSink`); their values are checked above
