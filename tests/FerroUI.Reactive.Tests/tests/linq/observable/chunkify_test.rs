//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ChunkifyTest.cs` of Rx.NET.
//! Adapted: the enumerable is a `PullEnumerable` of `Vec`s, whose enumerator gives `Ok` chunks and
//! the error of the source as its `Err` item.

use ferroui_reactive::linq::PullEnumerator;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;

type Enumerator = Rc<RefCell<Option<PullEnumerator<Vec<i32>>>>>;

/// The `log` action of upstream: a step that must give a chunk, which is recorded.
fn log(e: &Enumerator, res: &Rc<RefCell<Vec<Vec<i32>>>>) -> impl FnOnce() + 'static {
    let (e, res) = (e.clone(), res.clone());
    move || {
        let value = e.borrow_mut().as_mut().unwrap().next();
        let Some(Ok(value)) = value else { panic!("expected a chunk") };
        res.borrow_mut().push(value);
    }
}

// Not ported: Chunkify_ArgumentChecking - argument validation

#[test]
fn chunkify_regular1() {
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

    let ys = xs.as_observable().chunkify();

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
    assert_eq!(res[0], Vec::<i32>::new());
    assert_eq!(res[1], vec![3]);
    assert_eq!(res[2], vec![4]);
    assert_eq!(res[3], Vec::<i32>::new());
    assert_eq!(res[4], vec![5, 6, 7]);
    assert_eq!(res[5], vec![8]);
    assert_eq!(res[6], Vec::<i32>::new());
}

#[test]
fn chunkify_regular2() {
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

    let ys = xs.as_observable().chunkify();

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
    assert_eq!(res[0], vec![3, 4, 5]);
    assert_eq!(res[1], vec![6, 7, 8]);
}

#[test]
fn chunkify_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_next(600, 6),
        RT::on_error(700, ex.clone()),
    ]);

    let ys = xs.as_observable().chunkify();

    let e: Enumerator = Rc::new(RefCell::new(None));
    let res = Rc::new(RefCell::new(Vec::new()));

    scheduler.schedule_absolute_action(250, {
        let e = e.clone();
        move || *e.borrow_mut() = Some(ys.get_enumerator().unwrap())
    });
    for time in [270, 310, 450, 470] {
        scheduler.schedule_absolute_action(time, log(&e, &res));
    }
    scheduler.schedule_absolute_action(750, {
        let (e, ex) = (e.clone(), ex.clone());
        move || match e.borrow_mut().as_mut().unwrap().next() {
            Some(Err(error)) => assert!(Rc::ptr_eq(&ex, &error)),
            _ => panic!("expected the error"),
        }
    });

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(250, 700));

    let res = res.borrow();
    assert_eq!(4, res.len());
    assert_eq!(res[0], Vec::<i32>::new());
    assert_eq!(res[1], vec![3]);
    assert_eq!(res[2], vec![4]);
    assert_eq!(res[3], Vec::<i32>::new());
}
