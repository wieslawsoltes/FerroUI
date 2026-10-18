//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/WhereTest.cs` of Rx.NET.
//! Adapted: `Where((x, i) => ...)` is `where_with_index`; a predicate that throws is `try_where`
//! (`try_where_with_index`) returning `Err(ex)`. The counters the predicates update and the
//! variable `ys` that the scheduled actions assign are shared `Cell`s and `RefCell`s.

use ferroui_reactive::disposables::SerialDisposable;
use ferroui_reactive::{IDisposable, IObservable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn is_prime(i: i32) -> bool {
    if i <= 1 {
        return false;
    }

    let max = (i as f64).sqrt() as i32;
    for j in 2..=max {
        if i % j == 0 {
            return false;
        }
    }

    true
}

fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

fn increment(invoked: &Cell<i32>) {
    invoked.set(invoked.get() + 1);
}

#[test]
fn where_complete() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
        RT::on_next(610, 12),
        RT::on_error(620, new_error("ex")),
        RT::on_completed(630),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().where_(move |x: &i32| {
                increment(&invoked);
                is_prime(*x)
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_next(340, 5),
        RT::on_next(390, 7),
        RT::on_next(580, 11),
        RT::on_completed(600),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_eq!(9, invoked.get());
}

#[test]
fn where_true() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().where_(move |_: &i32| {
                increment(&invoked);
                true
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_eq!(9, invoked.get());
}

#[test]
fn where_false() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().where_(move |_: &i32| {
                increment(&invoked);
                false
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_completed(600),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_eq!(9, invoked.get());
}

#[test]
fn where_dispose() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start_disposed(
            move || {
                source.as_observable().where_(move |x: &i32| {
                    increment(&invoked);
                    is_prime(*x)
                })
            },
            400,
        )
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_next(340, 5),
        RT::on_next(390, 7),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_eq!(5, invoked.get());
}

#[test]
fn where_error() {
    let scheduler = TestScheduler::new();

    let invoked = counter();
    let ex = new_error("ex");


    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_error(600, ex.clone()),
        RT::on_next(610, 12),
        RT::on_error(620, new_error("ex")),
        RT::on_completed(630),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().where_(move |x: &i32| {
                increment(&invoked);
                is_prime(*x)
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_next(340, 5),
        RT::on_next(390, 7),
        RT::on_next(580, 11),
        RT::on_error(600, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_eq!(9, invoked.get());
}

#[test]
fn where_throw() {
    let scheduler = TestScheduler::new();

    let invoked = counter();
    let ex = new_error("ex");


    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
        RT::on_next(610, 12),
        RT::on_error(620, new_error("ex")),
        RT::on_completed(630),
    ]);

    let source = xs.clone();
    let res = {
        let (invoked, ex) = (invoked.clone(), ex.clone());
        scheduler.start(move || {
            source.as_observable().try_where(move |x: &i32| {
                increment(&invoked);
                if *x > 5 {
                    return Err(ex.clone());
                }
                Ok(is_prime(*x))
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_next(340, 5),
        RT::on_error(380, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 380));

    assert_eq!(4, invoked.get());
}

#[test]
fn where_dispose_in_predicate() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
        RT::on_next(610, 12),
        RT::on_error(620, new_error("ex")),
        RT::on_completed(630),
    ]);

    let res = scheduler.create_observer::<i32>();

    let d = SerialDisposable::new();
    let ys: Rc<RefCell<Option<Rc<dyn IObservable<i32>>>>> = Rc::new(RefCell::new(None));

    {
        let (ys, xs, invoked, d) = (ys.clone(), xs.clone(), invoked.clone(), d.clone());
        scheduler.schedule_absolute_action(RT::CREATED, move || {
            *ys.borrow_mut() = Some(xs.as_observable().where_(move |x: &i32| {
                increment(&invoked);
                if *x == 8 {
                    d.dispose();
                }

                is_prime(*x)
            }));
        });
    }

    {
        let (ys, d, res) = (ys.clone(), d.clone(), res.clone());
        scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || {
            let ys = ys.borrow().clone().expect("ys is created");
            d.set_disposable(Some(ys.subscribe(res)));
        });
    }

    {
        let d = d.clone();
        scheduler.schedule_absolute_action(RT::DISPOSED, move || d.dispose());
    }

    scheduler.run();

    assert_messages!(res.messages(), RT::on_next(230, 3), RT::on_next(340, 5), RT::on_next(390, 7));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));

    assert_eq!(6, invoked.get());
}

#[test]
fn where_where_optimization_regular() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().where_(|x| *x > 3).where_(|x| x % 2 == 0));

    assert_messages!(
        res.messages(),
        RT::on_next(270, 4),
        RT::on_next(380, 6),
        RT::on_next(450, 8),
        RT::on_next(560, 10),
        RT::on_completed(600),
    );
}

#[test]
fn where_where_optimization_second_predicate_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || {
        source.as_observable().where_(|x| *x > 3).try_where(|x| {
            if *x <= 3 {
                return Err(new_error("ex"));
            }

            Ok(x % 2 == 0)
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(270, 4),
        RT::on_next(380, 6),
        RT::on_next(450, 8),
        RT::on_next(560, 10),
        RT::on_completed(600),
    );
}

#[test]
fn where_index_complete() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
        RT::on_next(610, 12),
        RT::on_error(620, new_error("ex")),
        RT::on_completed(630),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().where_with_index(move |x: &i32, i: usize| {
                increment(&invoked);
                is_prime(*x + i as i32 * 10)
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_next(390, 7),
        RT::on_completed(600),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_eq!(9, invoked.get());
}

#[test]
fn where_index_true() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().where_with_index(move |_: &i32, _: usize| {
                increment(&invoked);
                true
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_eq!(9, invoked.get());
}

#[test]
fn where_index_false() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().where_with_index(move |_: &i32, _: usize| {
                increment(&invoked);
                false
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_completed(600),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_eq!(9, invoked.get());
}

#[test]
fn where_index_dispose() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start_disposed(
            move || {
                source.as_observable().where_with_index(move |x: &i32, i: usize| {
                    increment(&invoked);
                    is_prime(*x + i as i32 * 10)
                })
            },
            400,
        )
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_next(390, 7),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));

    assert_eq!(5, invoked.get());
}

#[test]
fn where_index_error() {
    let scheduler = TestScheduler::new();

    let invoked = counter();
    let ex = new_error("ex");


    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_error(600, ex.clone()),
        RT::on_next(610, 12),
        RT::on_error(620, new_error("ex")),
        RT::on_completed(630),
    ]);

    let source = xs.clone();
    let res = {
        let invoked = invoked.clone();
        scheduler.start(move || {
            source.as_observable().where_with_index(move |x: &i32, i: usize| {
                increment(&invoked);
                is_prime(*x + i as i32 * 10)
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_next(390, 7),
        RT::on_error(600, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_eq!(9, invoked.get());
}

#[test]
fn where_index_throw() {
    let scheduler = TestScheduler::new();

    let invoked = counter();
    let ex = new_error("ex");


    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
        RT::on_next(610, 12),
        RT::on_error(620, new_error("ex")),
        RT::on_completed(630),
    ]);

    let source = xs.clone();
    let res = {
        let (invoked, ex) = (invoked.clone(), ex.clone());
        scheduler.start(move || {
            source.as_observable().try_where_with_index(move |x: &i32, i: usize| {
                increment(&invoked);
                if *x > 5 {
                    return Err(ex.clone());
                }
                Ok(is_prime(*x + i as i32 * 10))
            })
        })
    };

    assert_messages!(
        res.messages(),
        RT::on_next(230, 3),
        RT::on_error(380, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 380));

    assert_eq!(4, invoked.get());
}

#[test]
fn where_index_dispose_in_predicate() {
    let scheduler = TestScheduler::new();

    let invoked = counter();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_next(450, 8),
        RT::on_next(470, 9),
        RT::on_next(560, 10),
        RT::on_next(580, 11),
        RT::on_completed(600),
        RT::on_next(610, 12),
        RT::on_error(620, new_error("ex")),
        RT::on_completed(630),
    ]);

    let res = scheduler.create_observer::<i32>();

    let d = SerialDisposable::new();
    let ys: Rc<RefCell<Option<Rc<dyn IObservable<i32>>>>> = Rc::new(RefCell::new(None));

    {
        let (ys, xs, invoked, d) = (ys.clone(), xs.clone(), invoked.clone(), d.clone());
        scheduler.schedule_absolute_action(RT::CREATED, move || {
            *ys.borrow_mut() = Some(xs.as_observable().where_with_index(move |x: &i32, i: usize| {
                increment(&invoked);
                if *x == 8 {
                    d.dispose();
                }

                is_prime(*x + i as i32 * 10)
            }));
        });
    }

    {
        let (ys, d, res) = (ys.clone(), d.clone(), res.clone());
        scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || {
            let ys = ys.borrow().clone().expect("ys is created");
            d.set_disposable(Some(ys.subscribe(res)));
        });
    }

    {
        let d = d.clone();
        scheduler.schedule_absolute_action(RT::DISPOSED, move || d.dispose());
    }

    scheduler.run();

    assert_messages!(res.messages(), RT::on_next(230, 3), RT::on_next(390, 7));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));

    assert_eq!(6, invoked.get());
}

#[test]
fn where_where1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_completed(400),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().where_(|x| *x > 3).where_(|x| *x < 6));

    assert_messages!(
        res.messages(),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn where_where2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_completed(400),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().where_with_index(|_, i| i >= 1).where_(|x| *x < 6));

    assert_messages!(
        res.messages(),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn where_where3() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_completed(400),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().where_(|x| *x > 3).where_with_index(|_, i| i < 2));

    assert_messages!(
        res.messages(),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn where_where4() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(180, 2),
        RT::on_next(230, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(380, 6),
        RT::on_next(390, 7),
        RT::on_completed(400),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().where_with_index(|_, i| i >= 1).where_with_index(|_, i| i < 2));

    assert_messages!(
        res.messages(),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_completed(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

// Not ported: Where_ArgumentChecking - argument validation
// Not ported: WhereIndex_ArgumentChecking - argument validation
