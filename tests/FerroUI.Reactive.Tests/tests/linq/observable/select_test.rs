//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SelectTest.cs` of Rx.NET.
//! Adapted: `Select((x, index) => ...)` is `select_with_index`; a selector that throws is
//! `try_select` (`try_select_with_index`) returning `Err(ex)`; an observer that throws
//! `InvalidOperationException` panics, checked with `assert_throws`. The counters the selectors
//! update are shared `Cell`s.

use ferroui_reactive::disposables::SerialDisposable;
use ferroui_reactive::{IDisposable, IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, assert_throws, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

fn increment(invoked: &Cell<i32>) -> i32 {
    invoked.set(invoked.get() + 1);
    invoked.get()
}

#[test]
fn select_throws() {
    assert_throws(|| {
        Observable::return_(1).select(|x| x).subscribe_next(|_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::throw::<i32>(new_error("ex")).select(|x| x).subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::empty::<i32>().select(|x| x).subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create_with_action(|_o: Rc<dyn IObserver<i32>>| -> fn() { panic!("InvalidOperationException") })
            .select(|x| x)
            .subscribe_ignore();
    });
}

#[test]
fn select_dispose_inside_selector() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(100, 1), RT::on_next(200, 2), RT::on_next(500, 3), RT::on_next(600, 4)]);

    let invoked = counter();

    let res = scheduler.create_observer::<i32>();

    let d = SerialDisposable::new();
    {
        let (invoked, inner, clock) = (invoked.clone(), d.clone(), scheduler.clone());
        let subscription = xs.as_observable().select(move |x: i32| {
            increment(&invoked);
            if clock.clock() > 400 {
                inner.dispose();
            }
            x
        }).subscribe(res.clone());
        d.set_disposable(Some(subscription));
    }

    {
        let d = d.clone();
        scheduler.schedule_absolute_action(RT::DISPOSED, move || d.dispose());
    }

    scheduler.run();

    assert_messages!(res.messages(), RT::on_next(100, 1), RT::on_next(200, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(0, 500));

    assert_eq!(3, invoked.get());
}

#[test]
fn select_completed() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(350, 5),
        RT::on_completed(400),
        RT::on_next(410, -1),
        RT::on_completed(420),
        RT::on_error(430, new_error("ex")),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().select(move |x: i32| {
                increment(&invoked);
                x + 1
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(290, 5),
        RT::on_next(350, 6),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_eq!(4, invoked.get());
}

#[test]
fn select_not_completed() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(350, 5),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().select(move |x: i32| {
                increment(&invoked);
                x + 1
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(290, 5),
        RT::on_next(350, 6),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));

    assert_eq!(4, invoked.get());
}

#[test]
fn select_error() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(350, 5),
        RT::on_error(400, ex.clone()),
        RT::on_next(410, -1),
        RT::on_completed(420),
        RT::on_error(430, new_error("ex")),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().select(move |x: i32| {
                increment(&invoked);
                x + 1
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(290, 5),
        RT::on_next(350, 6),
        RT::on_error(400, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_eq!(4, invoked.get());
}

#[test]
fn select_selector_throws() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(240, 3),
        RT::on_next(290, 4),
        RT::on_next(350, 5),
        RT::on_completed(400),
        RT::on_next(410, -1),
        RT::on_completed(420),
        RT::on_error(430, new_error("ex")),
    ]);

    let source = xs.clone();
    let res = {
        let (invoked, ex) = (invoked.clone(), ex.clone());
        scheduler.start(move || {
            source.as_observable().try_select(move |x: i32| {
                if increment(&invoked) == 3 {
                    return Err(ex.clone());
                }
                Ok(x + 1)
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_error(290, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 290));

    assert_eq!(3, invoked.get());
}

#[test]
fn select_with_index_throws() {
    assert_throws(|| {
        Observable::return_(1).select_with_index(|x, _index| x).subscribe_next(|_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::throw::<i32>(new_error("ex")).select_with_index(|x, _index| x).subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::empty::<i32>().select_with_index(|x, _index| x).subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create_with_action(|_o: Rc<dyn IObserver<i32>>| -> fn() { panic!("InvalidOperationException") })
            .select_with_index(|x, _index| x)
            .subscribe_ignore();
    });
}

#[test]
fn select_with_index_dispose_inside_selector() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(100, 4), RT::on_next(200, 3), RT::on_next(500, 2), RT::on_next(600, 1)]);

    let invoked = counter();

    let res = scheduler.create_observer::<i32>();

    let d = SerialDisposable::new();
    {
        let (invoked, inner, clock) = (invoked.clone(), d.clone(), scheduler.clone());
        let subscription = xs.as_observable().select_with_index(move |x: i32, index: usize| {
            increment(&invoked);
            if clock.clock() > 400 {
                inner.dispose();
            }
            x + index as i32 * 10
        }).subscribe(res.clone());
        d.set_disposable(Some(subscription));
    }

    {
        let d = d.clone();
        scheduler.schedule_absolute_action(RT::DISPOSED, move || d.dispose());
    }

    scheduler.run();

    assert_messages!(res.messages(), RT::on_next(100, 4), RT::on_next(200, 13));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(0, 500));

    assert_eq!(3, invoked.get());
}

#[test]
fn select_with_index_completed() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 5),
        RT::on_next(210, 4),
        RT::on_next(240, 3),
        RT::on_next(290, 2),
        RT::on_next(350, 1),
        RT::on_completed(400),
        RT::on_next(410, -1),
        RT::on_completed(420),
        RT::on_error(430, new_error("ex")),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().select_with_index(move |x: i32, index: usize| {
                increment(&invoked);
                (x + 1) + (index as i32 * 10)
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(210, 5),
        RT::on_next(240, 14),
        RT::on_next(290, 23),
        RT::on_next(350, 32),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_eq!(4, invoked.get());
}

#[test]
fn select_with_index_not_completed() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 5),
        RT::on_next(210, 4),
        RT::on_next(240, 3),
        RT::on_next(290, 2),
        RT::on_next(350, 1),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().select_with_index(move |x: i32, index: usize| {
                increment(&invoked);
                (x + 1) + (index as i32 * 10)
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(210, 5),
        RT::on_next(240, 14),
        RT::on_next(290, 23),
        RT::on_next(350, 32),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));

    assert_eq!(4, invoked.get());
}

#[test]
fn select_with_index_error() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 5),
        RT::on_next(210, 4),
        RT::on_next(240, 3),
        RT::on_next(290, 2),
        RT::on_next(350, 1),
        RT::on_error(400, ex.clone()),
        RT::on_next(410, -1),
        RT::on_completed(420),
        RT::on_error(430, new_error("ex")),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().select_with_index(move |x: i32, index: usize| {
                increment(&invoked);
                (x + 1) + (index as i32 * 10)
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(210, 5),
        RT::on_next(240, 14),
        RT::on_next(290, 23),
        RT::on_next(350, 32),
        RT::on_error(400, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_eq!(4, invoked.get());
}

#[test]
fn select_with_index_selector_throws() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 5),
        RT::on_next(210, 4),
        RT::on_next(240, 3),
        RT::on_next(290, 2),
        RT::on_next(350, 1),
        RT::on_completed(400),
        RT::on_next(410, -1),
        RT::on_completed(420),
        RT::on_error(430, new_error("ex")),
    ]);

    let source = xs.clone();
    let res = {
        let (invoked, ex) = (invoked.clone(), ex.clone());
        scheduler.start(move || {
            source.as_observable().try_select_with_index(move |x: i32, index: usize| {
                if increment(&invoked) == 3 {
                    return Err(ex.clone());
                }
                Ok((x + 1) + (index as i32 * 10))
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(210, 5),
        RT::on_next(240, 14),
        RT::on_error(290, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 290));

    assert_eq!(3, invoked.get());
}

#[test]
fn select_select1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 5),
        RT::on_next(210, 4),
        RT::on_next(240, 3),
        RT::on_next(290, 2),
        RT::on_next(350, 1),
        RT::on_completed(400),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().select(|x| x + 1).select(|x| x - 2));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 4 + 1 - 2),
        RT::on_next(240, 3 + 1 - 2),
        RT::on_next(290, 2 + 1 - 2),
        RT::on_next(350, 1 + 1 - 2),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn select_select2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 5),
        RT::on_next(210, 4),
        RT::on_next(240, 3),
        RT::on_next(290, 2),
        RT::on_next(350, 1),
        RT::on_completed(400),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().select_with_index(|x, i| x + i as i32).select(|x| x - 2));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 4 + 0 - 2),
        RT::on_next(240, 3 + 1 - 2),
        RT::on_next(290, 2 + 2 - 2),
        RT::on_next(350, 1 + 3 - 2),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn select_select3() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 5),
        RT::on_next(210, 4),
        RT::on_next(240, 3),
        RT::on_next(290, 2),
        RT::on_next(350, 1),
        RT::on_completed(400),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().select(|x| x + 1).select_with_index(|x, i| x - i as i32));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 4 + 1 - 0),
        RT::on_next(240, 3 + 1 - 1),
        RT::on_next(290, 2 + 1 - 2),
        RT::on_next(350, 1 + 1 - 3),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn select_select4() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 5),
        RT::on_next(210, 4),
        RT::on_next(240, 3),
        RT::on_next(290, 2),
        RT::on_next(350, 1),
        RT::on_completed(400),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().select_with_index(|x, i| x + i as i32).select_with_index(|x, i| x - i as i32));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 4),
        RT::on_next(240, 3),
        RT::on_next(290, 2),
        RT::on_next(350, 1),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

// Not ported: Select_ArgumentChecking - argument validation
// Not ported: SelectWithIndex_ArgumentChecking - argument validation
