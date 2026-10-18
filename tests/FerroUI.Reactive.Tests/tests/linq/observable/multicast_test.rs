//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/MulticastTest.cs` of Rx.NET.
//! Adapted: the variables the scheduled actions assign are shared slots; the subject and selector
//! that throw are the `try_multicast_with` form returning `Err`.

use ferroui_reactive::subjects::{IConnectableObservable, ISubjectHandle, Subject};
use ferroui_reactive::{IDisposable, IObservable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;

type Slot<T> = Rc<RefCell<Option<T>>>;

fn slot<T>() -> Slot<T> {
    Rc::new(RefCell::new(None))
}

fn get<T: Clone>(slot: &Slot<T>) -> T {
    slot.borrow().clone().expect("the slot is assigned")
}

/// `scheduler.ScheduleAbsolute(time, () => c = xs.Multicast(s))`.
fn multicast_at(scheduler: &TestScheduler, time: i64, c: &Slot<Rc<dyn IConnectableObservable<i32>>>, xs: Rc<dyn IObservable<i32>>, s: Rc<Subject<i32>>) {
    let c = c.clone();
    scheduler.schedule_absolute_action(time, move || {
        let subject: Rc<dyn ISubjectHandle<i32, i32>> = s.clone();
        *c.borrow_mut() = Some(xs.multicast(subject));
    });
}

/// `scheduler.ScheduleAbsolute(time, () => d = c.Connect())`.
fn connect_at(scheduler: &TestScheduler, time: i64, c: &Slot<Rc<dyn IConnectableObservable<i32>>>, d: &Slot<Rc<dyn IDisposable>>) {
    let (c, d) = (c.clone(), d.clone());
    scheduler.schedule_absolute_action(time, move || *d.borrow_mut() = Some(get(&c).connect()));
}

/// `scheduler.ScheduleAbsolute(time, () => d.Dispose())`.
fn dispose_at(scheduler: &TestScheduler, time: i64, d: &Slot<Rc<dyn IDisposable>>) {
    let d = d.clone();
    scheduler.schedule_absolute_action(time, move || get(&d).dispose());
}

#[test]
fn multicast_hot_1() {
    let scheduler = TestScheduler::new();

    let s = Subject::<i32>::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed(390),
    ]);

    let c: Slot<Rc<dyn IConnectableObservable<i32>>> = slot();
    let o = scheduler.create_observer::<i32>();
    let d1: Slot<Rc<dyn IDisposable>> = slot();
    let d2: Slot<Rc<dyn IDisposable>> = slot();

    multicast_at(&scheduler, 50, &c, xs.as_observable(), s.clone());
    {
        let (c, d1, o) = (c.clone(), d1.clone(), o.clone());
        scheduler.schedule_absolute_action(100, move || *d1.borrow_mut() = Some(get(&c).subscribe(o)));
    }
    connect_at(&scheduler, 200, &c, &d2);
    dispose_at(&scheduler, 300, &d1);

    scheduler.run();

    assert_messages!(o.messages(), RT::on_next(210, 3), RT::on_next(240, 4), RT::on_next(270, 5));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));
}

#[test]
fn multicast_hot_2() {
    let scheduler = TestScheduler::new();

    let s = Subject::<i32>::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed(390),
    ]);

    let c: Slot<Rc<dyn IConnectableObservable<i32>>> = slot();
    let o = scheduler.create_observer::<i32>();
    let d1: Slot<Rc<dyn IDisposable>> = slot();
    let d2: Slot<Rc<dyn IDisposable>> = slot();

    multicast_at(&scheduler, 50, &c, xs.as_observable(), s.clone());
    connect_at(&scheduler, 100, &c, &d2);
    {
        let (c, d1, o) = (c.clone(), d1.clone(), o.clone());
        scheduler.schedule_absolute_action(200, move || *d1.borrow_mut() = Some(get(&c).subscribe(o)));
    }
    dispose_at(&scheduler, 300, &d1);

    scheduler.run();

    assert_messages!(o.messages(), RT::on_next(210, 3), RT::on_next(240, 4), RT::on_next(270, 5));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(100, 390));
}

#[test]
fn multicast_hot_3() {
    let scheduler = TestScheduler::new();

    let s = Subject::<i32>::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed(390),
    ]);

    let c: Slot<Rc<dyn IConnectableObservable<i32>>> = slot();
    let o = scheduler.create_observer::<i32>();
    let d1: Slot<Rc<dyn IDisposable>> = slot();
    let d2: Slot<Rc<dyn IDisposable>> = slot();

    multicast_at(&scheduler, 50, &c, xs.as_observable(), s.clone());
    connect_at(&scheduler, 100, &c, &d2);
    {
        let (c, d1, o) = (c.clone(), d1.clone(), o.clone());
        scheduler.schedule_absolute_action(200, move || *d1.borrow_mut() = Some(get(&c).subscribe(o)));
    }
    dispose_at(&scheduler, 300, &d2);
    connect_at(&scheduler, 335, &c, &d2);

    scheduler.run();

    assert_messages!(
        o.messages(),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(340, 7),
        RT::on_completed::<i32>(390),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(100, 300), RT::subscribe(335, 390));
}

#[test]
fn multicast_hot_4() {
    let scheduler = TestScheduler::new();

    let s = Subject::<i32>::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_error(390, ex.clone()),
    ]);

    let c: Slot<Rc<dyn IConnectableObservable<i32>>> = slot();
    let o = scheduler.create_observer::<i32>();
    let d1: Slot<Rc<dyn IDisposable>> = slot();
    let d2: Slot<Rc<dyn IDisposable>> = slot();

    multicast_at(&scheduler, 50, &c, xs.as_observable(), s.clone());
    connect_at(&scheduler, 100, &c, &d2);
    {
        let (c, d1, o) = (c.clone(), d1.clone(), o.clone());
        scheduler.schedule_absolute_action(200, move || *d1.borrow_mut() = Some(get(&c).subscribe(o)));
    }
    dispose_at(&scheduler, 300, &d2);
    connect_at(&scheduler, 335, &c, &d2);

    scheduler.run();

    assert_messages!(
        o.messages(),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(340, 7),
        RT::on_error::<i32>(390, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(100, 300), RT::subscribe(335, 390));
}

#[test]
fn multicast_hot_5() {
    let scheduler = TestScheduler::new();

    let s = Subject::<i32>::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_error(390, ex.clone()),
    ]);

    let c: Slot<Rc<dyn IConnectableObservable<i32>>> = slot();
    let o = scheduler.create_observer::<i32>();
    let d1: Slot<Rc<dyn IDisposable>> = slot();
    let d2: Slot<Rc<dyn IDisposable>> = slot();

    multicast_at(&scheduler, 50, &c, xs.as_observable(), s.clone());
    connect_at(&scheduler, 100, &c, &d2);
    {
        let (c, d1, o) = (c.clone(), d1.clone(), o.clone());
        scheduler.schedule_absolute_action(400, move || *d1.borrow_mut() = Some(get(&c).subscribe(o)));
    }

    scheduler.run();

    assert_messages!(o.messages(), RT::on_error::<i32>(400, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(100, 390));
}

#[test]
fn multicast_hot_6() {
    let scheduler = TestScheduler::new();

    let s = Subject::<i32>::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed(390),
    ]);

    let c: Slot<Rc<dyn IConnectableObservable<i32>>> = slot();
    let o = scheduler.create_observer::<i32>();
    let d1: Slot<Rc<dyn IDisposable>> = slot();
    let d2: Slot<Rc<dyn IDisposable>> = slot();

    multicast_at(&scheduler, 50, &c, xs.as_observable(), s.clone());
    connect_at(&scheduler, 100, &c, &d2);
    {
        let (c, d1, o) = (c.clone(), d1.clone(), o.clone());
        scheduler.schedule_absolute_action(400, move || *d1.borrow_mut() = Some(get(&c).subscribe(o)));
    }

    scheduler.run();

    assert_messages!(o.messages(), RT::on_completed::<i32>(400));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(100, 390));
}

#[test]
fn multicast_cold_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed(390),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || {
            xs.as_observable().multicast_with(
                || {
                    let subject: Rc<dyn ISubjectHandle<i32, i32>> = Subject::new();
                    subject
                },
                |ys| ys,
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed::<i32>(390),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));
}

#[test]
fn multicast_cold_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_error(390, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || {
            xs.as_observable().multicast_with(
                || {
                    let subject: Rc<dyn ISubjectHandle<i32, i32>> = Subject::new();
                    subject
                },
                |ys| ys,
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_error::<i32>(390, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));
}

#[test]
fn multicast_cold_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || {
            xs.as_observable().multicast_with(
                || {
                    let subject: Rc<dyn ISubjectHandle<i32, i32>> = Subject::new();
                    subject
                },
                |ys| ys,
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn multicast_cold_zip() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(40, 0),
        RT::on_next(90, 1),
        RT::on_next(150, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(330, 6),
        RT::on_next(340, 7),
        RT::on_completed(390),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || {
            xs.as_observable().multicast_with(
                || {
                    let subject: Rc<dyn ISubjectHandle<i32, i32>> = Subject::new();
                    subject
                },
                |ys| ys.zip(ys.clone(), |a, b| a + b),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 6),
        RT::on_next(240, 8),
        RT::on_next(270, 10),
        RT::on_next(330, 12),
        RT::on_next(340, 14),
        RT::on_completed(390),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 390));
}

#[test]
fn multicast_subject_selector_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(240, 2), RT::on_completed(300)]);

    let res = scheduler.start({
        let (xs, ex) = (xs.clone(), ex.clone());
        move || {
            xs.as_observable().try_multicast_with(
                move || -> Result<Rc<dyn ISubjectHandle<i32, i32>>, ObservableError> { Err(ex.clone()) },
                |ys: Rc<dyn IObservable<i32>>| -> Result<Rc<dyn IObservable<i32>>, ObservableError> { Ok(ys) },
            )
        }
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex.clone()));

    assert_subscriptions!(xs.subscriptions());
}

#[test]
fn multicast_selector_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(240, 2), RT::on_completed(300)]);

    let res = scheduler.start({
        let (xs, ex) = (xs.clone(), ex.clone());
        move || {
            xs.as_observable().try_multicast_with(
                || -> Result<Rc<dyn ISubjectHandle<i32, i32>>, ObservableError> {
                    let subject: Rc<dyn ISubjectHandle<i32, i32>> = Subject::new();
                    Ok(subject)
                },
                move |_: Rc<dyn IObservable<i32>>| -> Result<Rc<dyn IObservable<i32>>, ObservableError> { Err(ex.clone()) },
            )
        }
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex.clone()));

    assert_subscriptions!(xs.subscriptions());
}

// Not ported: Multicast_ArgumentChecking - argument validation
