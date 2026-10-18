//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RepeatWhenTest.cs` of Rx.NET.
//! Adapted: the signals of the completions are `Unit` values (upstream's `object`); a handler or a
//! selector that throws is the `try_` form returning the error; an exception thrown by an observer
//! is a panic, checked with `assert_throws`.

use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::{IDisposable, IObservable, IObserver, Observable, ObservableExt, Unit};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, assert_throws, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn repeat_when_handler_crash() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(10)]);

    let ex = new_error("InvalidOperationException");

    let res = scheduler.start({
        let (xs, ex) = (xs.clone(), ex.clone());
        move || xs.as_observable().try_repeat_when(move |_| Err::<Rc<dyn IObservable<Unit>>, _>(ex.clone()))
    });

    assert_messages!(res.messages(), RT::on_error(200, ex.clone()));

    assert_subscriptions!(xs.subscriptions());
}

#[test]
fn repeat_when_handler_error() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(10)]);

    let ex = new_error("InvalidOperationException");

    let res = scheduler.start({
        let (xs, ex) = (xs.clone(), ex.clone());
        move || {
            xs.as_observable().repeat_when(move |v| {
                let ex = ex.clone();
                v.try_select(move |_| Err::<Unit, _>(ex.clone()))
            })
        }
    });

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn repeat_when_disposed() {
    let main = Subject::<i32>::new();
    let inner = Subject::<i32>::new();

    let d = main
        .as_observable()
        .repeat_when({
            let inner = inner.clone();
            move |_| inner.as_observable()
        })
        .subscribe_ignore();

    assert!(main.has_observers());
    assert!(inner.has_observers());

    d.dispose();

    assert!(!main.has_observers());
    assert!(!inner.has_observers());
}

#[test]
fn repeat_when_handler_completed_disposes_main() {
    let main = Subject::<i32>::new();
    let inner = Subject::<i32>::new();

    let end = Rc::new(Cell::new(0));
    let items = Rc::new(Cell::new(0));
    let errors = Rc::new(Cell::new(0));

    let _d = main
        .as_observable()
        .repeat_when({
            let inner = inner.clone();
            move |_| inner.as_observable()
        })
        .subscribe_all(
            {
                let items = items.clone();
                move |_| items.set(items.get() + 1)
            },
            {
                let errors = errors.clone();
                move |_| errors.set(errors.get() + 1)
            },
            {
                let end = end.clone();
                move || end.set(end.get() + 1)
            },
        );

    assert!(main.has_observers());
    assert!(inner.has_observers());

    inner.on_completed();

    assert!(!main.has_observers());
    assert!(!inner.has_observers());

    assert_eq!(0, items.get());
    assert_eq!(0, errors.get());
    assert_eq!(1, end.get());
}

#[test]
fn repeat_when_handler_error_disposes_main() {
    let main = Subject::<i32>::new();
    let inner = Subject::<i32>::new();

    let end = Rc::new(Cell::new(0));
    let items = Rc::new(Cell::new(0));
    let errors = Rc::new(Cell::new(0));

    let _d = main
        .as_observable()
        .repeat_when({
            let inner = inner.clone();
            move |_| inner.as_observable()
        })
        .subscribe_all(
            {
                let items = items.clone();
                move |_| items.set(items.get() + 1)
            },
            {
                let errors = errors.clone();
                move |_| errors.set(errors.get() + 1)
            },
            {
                let end = end.clone();
                move || end.set(end.get() + 1)
            },
        );

    assert!(main.has_observers());
    assert!(inner.has_observers());

    inner.on_error(new_error("InvalidOperationException"));

    assert!(!main.has_observers());
    assert!(!inner.has_observers());

    assert_eq!(0, items.get());
    assert_eq!(1, errors.get());
    assert_eq!(0, end.get());
}

#[test]
fn repeat_when_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_completed(250)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_when(|v| v)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(300, 1),
        RT::on_next(350, 2),
        RT::on_next(400, 3),
        RT::on_next(550, 1),
        RT::on_next(600, 2),
        RT::on_next(650, 3),
        RT::on_next(800, 1),
        RT::on_next(850, 2),
        RT::on_next(900, 3)
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
fn repeat_when_infinite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_when(|v| v)
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn repeat_when_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_error(250, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_when(|v| v)
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_error(450, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn repeat_when_throws() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::return_on(1, scheduler1.scheduler()).repeat_when(|v| v);

    let _d1 = xs.subscribe_next(|_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler1.run());

    let scheduler2 = TestScheduler::new();

    let ys = Observable::throw_on::<i32>(new_error("ex"), scheduler2.scheduler()).repeat_when(|v| v);

    let _d2 = ys.subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler2.run());

    let scheduler3 = TestScheduler::new();

    let zs = Observable::return_on(1, scheduler3.scheduler()).repeat_when(|v| v);

    let d = zs.subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));

    scheduler3.schedule_absolute_action(210, move || d.dispose());

    scheduler3.run();

    let xss = Observable::create(|_o: Rc<dyn IObserver<i32>>| -> Rc<dyn IDisposable> { panic!("InvalidOperationException") }).repeat_when(|v| v);

    assert_throws(|| {
        xss.subscribe_ignore();
    });
}

// Not ported: RepeatWhen_ArgumentChecking - argument validation
// Not ported: RepeatWhen_Handler_Completed - needs operators Take and Skip (phase 3, later)
// Not ported: RepeatWhen_RepeatCount_Basic - needs operator TakeWhile (phase 3, later)
// Not ported: RepeatWhen_RepeatCount_Dispose - needs operator TakeWhile (phase 3, later)
// Not ported: RepeatWhen_RepeatCount_Infinite - needs operator TakeWhile (phase 3, later)
// Not ported: RepeatWhen_RepeatCount_Error - needs operator TakeWhile (phase 3, later)
// Not ported: RepeatWhen_RepeatCount_Throws - needs operator TakeWhile (phase 3, later)
// Not ported: RepeatWhen_Observable_Repeat_Delayed - needs operators SelectMany and Delay (phase 3, later)
