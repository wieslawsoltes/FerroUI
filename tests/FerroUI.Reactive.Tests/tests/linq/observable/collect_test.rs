//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/CollectTest.cs` of Rx.NET.
//! Adapted: the enumerable is a `PullEnumerable`, whose enumerator gives `Ok` values; a collector
//! or merge function that throws is the `try_collect`/`try_collect_with` form returning the
//! error, and the thrown error is the `Err` item of the step (or of `get_enumerator`).

use ferroui_reactive::linq::PullEnumerator;
use ferroui_reactive::{ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Enumerator = Rc<RefCell<Option<PullEnumerator<i32>>>>;

/// The `log` action of upstream: a step that must give a value, which is recorded.
fn log(e: &Enumerator, res: &Rc<RefCell<Vec<i32>>>) -> impl FnOnce() + 'static {
    let (e, res) = (e.clone(), res.clone());
    move || {
        let value = e.borrow_mut().as_mut().unwrap().next();
        let Some(Ok(value)) = value else { panic!("expected a value") };
        res.borrow_mut().push(value);
    }
}

/// A step that must fail; the error is recorded (`catch (Exception err) { ex_ = err; }`).
fn fail(e: &Enumerator, ex_: &Rc<RefCell<Option<ObservableError>>>) -> impl FnOnce() + 'static {
    let (e, ex_) = (e.clone(), ex_.clone());
    move || {
        if let Some(Err(error)) = e.borrow_mut().as_mut().unwrap().next() {
            *ex_.borrow_mut() = Some(error);
        }
    }
}

// Not ported: Collect_ArgumentChecking - argument validation

#[test]
fn collect_regular1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_next(600, 6),
        RT::on_next(700, 7),
        RT::on_next(800, 8),
        RT::on_completed(900),
    ]);

    let ys = xs.as_observable().collect(|| 0, |x, y| x + y);

    let e: Enumerator = Rc::new(RefCell::new(None));
    let res = Rc::new(RefCell::new(Vec::new()));

    scheduler.schedule_absolute_action(250, {
        let e = e.clone();
        move || *e.borrow_mut() = Some(ys.get_enumerator().unwrap())
    });
    for time in [270, 310, 450, 470, 750, 850, 950] {
        scheduler.schedule_absolute_action(time, log(&e, &res));
    }
    scheduler.schedule_absolute_action(980, {
        let e = e.clone();
        move || assert!(e.borrow_mut().as_mut().unwrap().next().is_none())
    });

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(250, 900));

    let res = res.borrow();
    assert_eq!(7, res.len());
    assert_eq!(res[0], 0);
    assert_eq!(res[1], 3);
    assert_eq!(res[2], 4);
    assert_eq!(res[3], 0);
    assert_eq!(res[4], 5 + 6 + 7);
    assert_eq!(res[5], 8);
    assert_eq!(res[6], 0);
}

#[test]
fn collect_regular2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_next(600, 6),
        RT::on_next(700, 7),
        RT::on_next(800, 8),
        RT::on_completed(900),
    ]);

    let ys = xs.as_observable().collect(|| 0, |x, y| x + y);

    let e: Enumerator = Rc::new(RefCell::new(None));
    let res = Rc::new(RefCell::new(Vec::new()));

    scheduler.schedule_absolute_action(250, {
        let e = e.clone();
        move || *e.borrow_mut() = Some(ys.get_enumerator().unwrap())
    });
    scheduler.schedule_absolute_action(550, log(&e, &res));
    scheduler.schedule_absolute_action(950, log(&e, &res));
    scheduler.schedule_absolute_action(980, {
        let e = e.clone();
        move || assert!(e.borrow_mut().as_mut().unwrap().next().is_none())
    });

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(250, 900));

    let res = res.borrow();
    assert_eq!(2, res.len());
    assert_eq!(res[0], 3 + 4 + 5);
    assert_eq!(res[1], 6 + 7 + 8);
}

#[test]
fn collect_initial_collector_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_completed(500),
    ]);

    let ex = new_error("ex");

    let e = ex.clone();
    let ys = xs.as_observable().try_collect_with(move || Err::<i32, _>(e.clone()), |x, y| Ok(x + y), |_| Ok(0));

    let ex_: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    scheduler.schedule_absolute_action(250, {
        let ex_ = ex_.clone();
        move || {
            if let Err(err) = ys.get_enumerator() {
                *ex_.borrow_mut() = Some(err);
            }
        }
    });

    scheduler.run();

    assert_subscriptions!(xs.subscriptions());

    assert!(Rc::ptr_eq(ex_.borrow().as_ref().unwrap(), &ex));
}

#[test]
fn collect_second_collector_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_completed(500),
    ]);

    let ex = new_error("ex");

    let n = Cell::new(0);
    let e = ex.clone();
    let ys = xs.as_observable().try_collect(
        move || {
            let first = n.get() == 0;
            n.set(n.get() + 1);
            if first {
                Ok(0)
            } else {
                Err(e.clone())
            }
        },
        |x, y| Ok(x + y),
    );

    let e: Enumerator = Rc::new(RefCell::new(None));
    let ex_: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    scheduler.schedule_absolute_action(250, {
        let e = e.clone();
        move || *e.borrow_mut() = Some(ys.get_enumerator().unwrap())
    });
    scheduler.schedule_absolute_action(350, fail(&e, &ex_));

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(250, 350));

    assert!(Rc::ptr_eq(ex_.borrow().as_ref().unwrap(), &ex));
}

#[test]
fn collect_new_collector_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_completed(500),
    ]);

    let ex = new_error("ex");

    let e = ex.clone();
    let ys = xs.as_observable().try_collect_with(|| Ok(0), |x, y| Ok(x + y), move |_| Err(e.clone()));

    let e: Enumerator = Rc::new(RefCell::new(None));
    let ex_: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    scheduler.schedule_absolute_action(250, {
        let e = e.clone();
        move || *e.borrow_mut() = Some(ys.get_enumerator().unwrap())
    });
    scheduler.schedule_absolute_action(350, fail(&e, &ex_));

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(250, 350));

    assert!(Rc::ptr_eq(ex_.borrow().as_ref().unwrap(), &ex));
}

#[test]
fn collect_merge_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_completed(500),
    ]);

    let ex = new_error("ex");

    let e = ex.clone();
    let ys = xs.as_observable().try_collect(|| Ok(0), move |_, _: i32| Err(e.clone()));

    let e: Enumerator = Rc::new(RefCell::new(None));
    let ex_: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    scheduler.schedule_absolute_action(250, {
        let e = e.clone();
        move || *e.borrow_mut() = Some(ys.get_enumerator().unwrap())
    });
    scheduler.schedule_absolute_action(350, fail(&e, &ex_));

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(250, 300));

    assert!(Rc::ptr_eq(ex_.borrow().as_ref().unwrap(), &ex));
}
