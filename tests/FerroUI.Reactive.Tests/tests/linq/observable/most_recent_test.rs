//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/MostRecentTest.cs` of Rx.NET.
//! Adapted: the enumerable is a `PullEnumerable`, whose enumerator gives `Ok` values; disposing an
//! enumerator is `PullEnumerator::dispose`.

use ferroui_reactive::linq::PullEnumerator;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_subscriptions, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;

type Enumerator = Rc<RefCell<Option<PullEnumerator<i32>>>>;

/// A step that must give a value, which is recorded in `o`.
fn step(e: &Enumerator, o: &Rc<RefCell<Vec<i32>>>) -> impl FnOnce() + 'static {
    let (e, o) = (e.clone(), o.clone());
    move || {
        let value = e.borrow_mut().as_mut().unwrap().next();
        let Some(Ok(value)) = value else { panic!("expected a value") };
        o.borrow_mut().push(value);
    }
}

// Not ported: MostRecent_ArgumentChecking - argument validation
// Not ported: MostRecent1 - real threads
// Not ported: MostRecent_Error - real threads

#[test]
fn most_recent2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_next(270, 7),
        RT::on_next(280, 8),
        RT::on_next(290, 9),
        RT::on_completed(300),
    ]);

    let res = xs.as_observable().most_recent(0);

    let e1: Enumerator = Rc::new(RefCell::new(None));
    scheduler.schedule_absolute_action(200, {
        let (e1, res) = (e1.clone(), res.clone());
        move || *e1.borrow_mut() = Some(res.get_enumerator().unwrap())
    });

    let o1 = Rc::new(RefCell::new(Vec::new()));
    for time in [205, 232, 234, 265] {
        scheduler.schedule_absolute_action(time, step(&e1, &o1));
    }
    scheduler.schedule_absolute_action(285, {
        let e1 = e1.clone();
        move || e1.borrow().as_ref().unwrap().dispose()
    });

    let e2: Enumerator = Rc::new(RefCell::new(None));
    scheduler.schedule_absolute_action(255, {
        let (e2, res) = (e2.clone(), res.clone());
        move || *e2.borrow_mut() = Some(res.get_enumerator().unwrap())
    });

    let o2 = Rc::new(RefCell::new(Vec::new()));
    for time in [258, 262, 264, 275] {
        scheduler.schedule_absolute_action(time, step(&e2, &o2));
    }

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 285), RT::subscribe(255, 300));

    assert_eq!(*o1.borrow(), vec![0, 3, 3, 6]);
    assert_eq!(*o2.borrow(), vec![0, 6, 6, 7]);
}
