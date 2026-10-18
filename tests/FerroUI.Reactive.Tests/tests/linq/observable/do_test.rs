//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DoTest.cs` of Rx.NET.
//! Adapted: the overloads of `Do` are `do_` (`Action<T>`), `do_with_completed` (`Action<T>,
//! Action`), `do_with_error` (`Action<T>, Action<Exception>`), `do_all` (the three actions) and
//! `do_observer` (`IObserver<T>`); an action that throws is the `try_` form of the overload
//! returning `Err(ex)`. The counters the actions update are shared `Cell`s.

use ferroui_reactive::{ObservableError, ObservableExt, Observer};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

fn same_error(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

fn counter(value: i32) -> Rc<Cell<i32>> {
    Rc::new(Cell::new(value))
}

fn flag() -> Rc<Cell<bool>> {
    Rc::new(Cell::new(false))
}

#[test]
fn do_should_see_all_values() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let i = counter(0);
    let sum = counter(2 + 3 + 4 + 5);
    let source = xs.clone();
    let res = {
        let (i, sum) = (i.clone(), sum.clone());
        scheduler.start(move || {
            source.as_observable().do_(move |x: &i32| {
                i.set(i.get() + 1);
                sum.set(sum.get() - x);
            })
        })
    };

    assert_eq!(4, i.get());
    assert_eq!(0, sum.get());

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do_plain_action() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let i = counter(0);
    let source = xs.clone();
    let res = {
        let i = i.clone();
        scheduler.start(move || source.as_observable().do_(move |_: &i32| i.set(i.get() + 1)))
    };

    assert_eq!(4, i.get());

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do_next_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let i = counter(0);
    let sum = counter(2 + 3 + 4 + 5);
    let completed = flag();
    let source = xs.clone();
    let res = {
        let (i, sum, completed) = (i.clone(), sum.clone(), completed.clone());
        scheduler.start(move || {
            source.as_observable().do_with_completed(
                move |x: &i32| {
                    i.set(i.get() + 1);
                    sum.set(sum.get() - x);
                },
                move || completed.set(true),
            )
        })
    };

    assert_eq!(4, i.get());
    assert_eq!(0, sum.get());
    assert!(completed.get());

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do_next_completed_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let i = counter(0);
    let completed = flag();
    let source = xs.clone();
    let res = {
        let (i, completed) = (i.clone(), completed.clone());
        scheduler.start(move || source.as_observable().do_with_completed(move |_: &i32| i.set(i.get() + 1), move || completed.set(true)))
    };

    assert_eq!(0, i.get());
    assert!(!completed.get());

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn do_next_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_error(250, ex.clone()),
    ]);

    let i = counter(0);
    let sum = counter(2 + 3 + 4 + 5);
    let saw_error = flag();
    let source = xs.clone();
    let res = {
        let (i, sum, saw_error, ex) = (i.clone(), sum.clone(), saw_error.clone(), ex.clone());
        scheduler.start(move || {
            source.as_observable().do_with_error(
                move |x: &i32| {
                    i.set(i.get() + 1);
                    sum.set(sum.get() - x);
                },
                move |e: ObservableError| saw_error.set(same_error(&e, &ex)),
            )
        })
    };

    assert_eq!(4, i.get());
    assert_eq!(0, sum.get());
    assert!(saw_error.get());

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_error(250, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do_next_error_not() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let i = counter(0);
    let sum = counter(2 + 3 + 4 + 5);
    let saw_error = flag();
    let source = xs.clone();
    let res = {
        let (i, sum, saw_error) = (i.clone(), sum.clone(), saw_error.clone());
        scheduler.start(move || {
            source.as_observable().do_with_error(
                move |x: &i32| {
                    i.set(i.get() + 1);
                    sum.set(sum.get() - x);
                },
                move |_: ObservableError| saw_error.set(true),
            )
        })
    };

    assert_eq!(4, i.get());
    assert_eq!(0, sum.get());
    assert!(!saw_error.get());

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do_next_error_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let i = counter(0);
    let sum = counter(2 + 3 + 4 + 5);
    let saw_error = flag();
    let has_completed = flag();
    let source = xs.clone();
    let res = {
        let (i, sum, saw_error, has_completed) = (i.clone(), sum.clone(), saw_error.clone(), has_completed.clone());
        scheduler.start(move || {
            source.as_observable().do_all(
                move |x: &i32| {
                    i.set(i.get() + 1);
                    sum.set(sum.get() - x);
                },
                move |_: ObservableError| saw_error.set(true),
                move || has_completed.set(true),
            )
        })
    };

    assert_eq!(4, i.get());
    assert_eq!(0, sum.get());
    assert!(!saw_error.get());
    assert!(has_completed.get());

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do_next_error_completed_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_error(250, ex.clone()),
    ]);

    let i = counter(0);
    let sum = counter(2 + 3 + 4 + 5);
    let saw_error = flag();
    let has_completed = flag();
    let source = xs.clone();
    let res = {
        let (i, sum, saw_error, has_completed, ex) = (i.clone(), sum.clone(), saw_error.clone(), has_completed.clone(), ex.clone());
        scheduler.start(move || {
            source.as_observable().do_all(
                move |x: &i32| {
                    i.set(i.get() + 1);
                    sum.set(sum.get() - x);
                },
                move |e: ObservableError| saw_error.set(same_error(&e, &ex)),
                move || has_completed.set(true),
            )
        })
    };

    assert_eq!(4, i.get());
    assert_eq!(0, sum.get());
    assert!(saw_error.get());
    assert!(!has_completed.get());

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_error(250, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do_next_error_completed_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let i = counter(0);
    let saw_error = flag();
    let has_completed = flag();
    let source = xs.clone();
    let res = {
        let (i, saw_error, has_completed) = (i.clone(), saw_error.clone(), has_completed.clone());
        scheduler.start(move || {
            source.as_observable().do_all(
                move |_: &i32| i.set(i.get() + 1),
                move |_: ObservableError| saw_error.set(true),
                move || has_completed.set(true),
            )
        })
    };

    assert_eq!(0, i.get());
    assert!(!saw_error.get());
    assert!(!has_completed.get());

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn do_observer_some_data_with_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_error(250, ex.clone()),
    ]);

    let i = counter(0);
    let sum = counter(2 + 3 + 4 + 5);
    let saw_error = flag();
    let has_completed = flag();
    let source = xs.clone();
    let res = {
        let (i, sum, saw_error, has_completed, ex) = (i.clone(), sum.clone(), saw_error.clone(), has_completed.clone(), ex.clone());
        scheduler.start(move || {
            source.as_observable().do_observer(Observer::create_all(
                move |x: i32| {
                    i.set(i.get() + 1);
                    sum.set(sum.get() - x);
                },
                move |e: ObservableError| saw_error.set(same_error(&e, &ex)),
                move || has_completed.set(true),
            ))
        })
    };

    assert_eq!(4, i.get());
    assert_eq!(0, sum.get());
    assert!(saw_error.get());
    assert!(!has_completed.get());

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_error(250, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do_observer_some_data_without_error() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    ]);

    let i = counter(0);
    let sum = counter(2 + 3 + 4 + 5);
    let saw_error = flag();
    let has_completed = flag();
    let source = xs.clone();
    let res = {
        let (i, sum, saw_error, has_completed) = (i.clone(), sum.clone(), saw_error.clone(), has_completed.clone());
        scheduler.start(move || {
            source.as_observable().do_observer(Observer::create_all(
                move |x: i32| {
                    i.set(i.get() + 1);
                    sum.set(sum.get() - x);
                },
                move |_: ObservableError| saw_error.set(true),
                move || has_completed.set(true),
            ))
        })
    };

    assert_eq!(4, i.get());
    assert_eq!(0, sum.get());
    assert!(!saw_error.get());
    assert!(has_completed.get());

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do1422_next_next_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let source = xs.clone();
    let res = {
        let ex = ex.clone();
        scheduler.start(move || source.as_observable().try_do(move |_: &i32| Err(ex.clone())))
    };

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn do1422_next_completed_next_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let source = xs.clone();
    let res = {
        let ex = ex.clone();
        scheduler.start(move || source.as_observable().try_do_with_completed(move |_: &i32| Err(ex.clone()), || Ok(())))
    };

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn do1422_next_completed_completed_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let source = xs.clone();
    let res = {
        let ex = ex.clone();
        scheduler.start(move || source.as_observable().try_do_with_completed(|_: &i32| Ok(()), move || Err(ex.clone())))
    };

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_error(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn do1422_next_error_next_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let source = xs.clone();
    let res = {
        let ex = ex.clone();
        scheduler.start(move || source.as_observable().try_do_with_error(move |_: &i32| Err(ex.clone()), |_: ObservableError| Ok(())))
    };

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn do1422_next_error_error_throws() {
    let scheduler = TestScheduler::new();

    let ex1 = new_error("ex1");
    let ex2 = new_error("ex2");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(210, ex1.clone())]);

    let source = xs.clone();
    let res = {
        let ex2 = ex2.clone();
        scheduler.start(move || source.as_observable().try_do_with_error(|_: &i32| Ok(()), move |_: ObservableError| Err(ex2.clone())))
    };

    assert_messages!(res.messages(), RT::on_error(210, ex2.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn do1422_next_error_completed_next_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let source = xs.clone();
    let res = {
        let ex = ex.clone();
        scheduler.start(move || source.as_observable().try_do_all(move |_: &i32| Err(ex.clone()), |_: ObservableError| Ok(()), || Ok(())))
    };

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn do1422_next_error_completed_error_throws() {
    let scheduler = TestScheduler::new();

    let ex1 = new_error("ex1");
    let ex2 = new_error("ex2");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(210, ex1.clone())]);

    let source = xs.clone();
    let res = {
        let ex2 = ex2.clone();
        scheduler.start(move || source.as_observable().try_do_all(|_: &i32| Ok(()), move |_: ObservableError| Err(ex2.clone()), || Ok(())))
    };

    assert_messages!(res.messages(), RT::on_error(210, ex2.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn do1422_next_error_completed_completed_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);

    let source = xs.clone();
    let res = {
        let ex = ex.clone();
        scheduler.start(move || source.as_observable().try_do_all(|_: &i32| Ok(()), |_: ObservableError| Ok(()), move || Err(ex.clone())))
    };

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_error(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

// Not ported: Do_ArgumentChecking - argument validation
// Not ported: Do1422_Observer_NextThrows - an observer of the port cannot fail (the methods of `IObserver` return nothing); actions that fail are the `try_do*` forms, tested above
// Not ported: Do1422_Observer_ErrorThrows - an observer of the port cannot fail (the methods of `IObserver` return nothing); actions that fail are the `try_do*` forms, tested above
// Not ported: Do1422_Observer_CompletedThrows - an observer of the port cannot fail (the methods of `IObserver` return nothing); actions that fail are the `try_do*` forms, tested above
