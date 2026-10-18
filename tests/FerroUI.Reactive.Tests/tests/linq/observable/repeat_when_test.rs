//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RepeatWhenTest.cs` of Rx.NET.
//! Adapted: the signals of the completions are `Unit` values (upstream's `object`); a handler or a
//! selector that throws is the `try_` form returning the error; an exception thrown by an observer
//! is a panic, checked with `assert_throws`.

use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::{IDisposable, IObservable, IObserver, Observable, ObservableExt, Unit};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, assert_throws, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
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
fn repeat_when_handler_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(10)]);

    let _ex = new_error("InvalidOperationException");

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_when(|v| v.take(1).skip(1))
    });

    assert_messages!(res.messages(), RT::on_completed(210));

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

/// `v => { var count = 0; return v.TakeWhile(w => ++count < limit); }`.
fn take_while_count(v: Rc<dyn IObservable<Unit>>, limit: i32) -> Rc<dyn IObservable<Unit>> {
    let count = Cell::new(0);
    v.take_while(move |_w| {
        count.set(count.get() + 1);
        count.get() < limit
    })
}

#[test]
fn repeat_when_repeat_count_basic() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(5, 1), RT::on_next(10, 2), RT::on_next(15, 3), RT::on_completed(20)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_when(|v| take_while_count(v, 3))
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
        RT::on_completed(260),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220), RT::subscribe(220, 240), RT::subscribe(240, 260));
}

#[test]
fn repeat_when_repeat_count_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(5, 1), RT::on_next(10, 2), RT::on_next(15, 3), RT::on_completed(20)]);

    let res = scheduler.start_disposed(
        {
            let xs = xs.clone();
            move || xs.as_observable().repeat_when(|v| take_while_count(v, 3))
        },
        231,
    );

    assert_messages!(res.messages(), RT::on_next(205, 1), RT::on_next(210, 2), RT::on_next(215, 3), RT::on_next(225, 1), RT::on_next(230, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220), RT::subscribe(220, 231));
}

#[test]
fn repeat_when_repeat_count_infinite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3)]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_when(|v| take_while_count(v, 3))
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn repeat_when_repeat_count_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 1), RT::on_next(150, 2), RT::on_next(200, 3), RT::on_error(250, ex.clone())]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().repeat_when(|v| take_while_count(v, 3))
    });

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(350, 2), RT::on_next(400, 3), RT::on_error(450, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn repeat_when_repeat_count_throws() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::return_on(1, scheduler1.scheduler()).repeat_when(|v| take_while_count(v, 3));

    let _d1 = xs.subscribe_next(|_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler1.run());

    let scheduler2 = TestScheduler::new();

    let ys = Observable::throw_on::<i32>(new_error("ex"), scheduler2.scheduler()).repeat_when(|v| take_while_count(v, 3));

    let _d2 = ys.subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler2.run());

    let scheduler3 = TestScheduler::new();

    let zs = Observable::return_on(1, scheduler3.scheduler()).repeat_when(|v| take_while_count(v, 100));

    let d = zs.subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));

    scheduler3.schedule_absolute_action(10, move || d.dispose());

    scheduler3.run();

    let xss = Observable::create(|_o: Rc<dyn IObserver<i32>>| -> Rc<dyn IDisposable> { panic!("InvalidOperationException") })
        .repeat_when(|v| take_while_count(v, 3));

    assert_throws(|| {
        xss.subscribe_ignore();
    });
}

#[test]
fn repeat_when_observable_repeat_delayed() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(5, 1), RT::on_next(10, 2), RT::on_next(15, 3), RT::on_completed(20)]);

    let res = scheduler.start({
        let (xs, ex, sch) = (xs.clone(), ex.clone(), scheduler.scheduler());
        move || {
            xs.as_observable().repeat_when(move |v| {
                let count = Rc::new(Cell::new(0));
                let (ex, sch) = (ex.clone(), sch.clone());
                v.select_many(move |_w| {
                    count.set(count.get() + 1);
                    let c = count.get();
                    if c == 3 {
                        return Observable::throw::<i32>(ex.clone());
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

// Not ported: RepeatWhen_ArgumentChecking - argument validation
