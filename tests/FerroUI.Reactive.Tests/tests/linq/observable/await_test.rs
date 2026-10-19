//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AwaitTest.cs` of Rx.NET.
//! Adapted: `GetAwaiter` gives the task of its `AsyncSubject<T>` (the subject's awaiter), so
//! `awaiter.OnCompleted(continuation)` is `task.continue_with(continuation)` and
//! `awaiter.GetResult()` is `task.result()`; the `InvalidOperationException` of a sequence without
//! a value is `SequenceError::NoElements`. `SynchronizationContext.SetSynchronizationContext(null)`
//! has no counterpart: the continuations of a task run where it ends. The variables that
//! scheduled actions assign are shared cells.

use ferroui_reactive::disposables::StableCompositeDisposable;
use ferroui_reactive::linq::SequenceError;
use ferroui_reactive::{ConnectableObservableExt, IDisposable, IObserver, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::tasks::Task;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The awaiter that a scheduled action assigns (`var awaiter = default(AsyncSubject<int>);`).
type Awaiter = Rc<RefCell<Option<Task<i32>>>>;

fn awaiter() -> Awaiter {
    Rc::new(RefCell::new(None))
}

fn get(awaiter: &Awaiter) -> Task<i32> {
    awaiter.borrow().clone().expect("the awaiter has been assigned")
}

/// `Assert.Same` of two errors.
fn same(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

#[test]
fn await_() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(20, -1),
        RT::on_next(150, 0),
        RT::on_next(220, 1),
        RT::on_next(290, 2),
        RT::on_next(340, 3),
        RT::on_completed(410),
    ]);

    let awaiter = awaiter();
    let result = Rc::new(Cell::new(0));
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let (xs, awaiter) = (xs.clone(), awaiter.clone());
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(xs.as_observable().get_awaiter());
        });
    }
    {
        let (scheduler_, awaiter, result, t) = (scheduler.clone(), awaiter.clone(), result.clone(), t.clone());
        scheduler.schedule_absolute_action(200, move || {
            get(&awaiter).continue_with(move |task| {
                t.set(scheduler_.clock());
                result.set(task.result().unwrap());
            });
        });
    }

    scheduler.run();

    assert_eq!(410, t.get());
    assert_eq!(3, result.get());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe_from(100));
}

#[test]
fn await_connectable() {
    let scheduler = TestScheduler::new();

    let s = Rc::new(Cell::new(0));

    let xs = {
        let (scheduler, s) = (scheduler.clone(), s.clone());
        Observable::create(move |observer: Rc<dyn IObserver<i32>>| {
            s.set(scheduler.clock());

            let on_next = observer.clone();
            let composite: Rc<dyn IDisposable> = StableCompositeDisposable::create(
                scheduler.schedule_absolute_action(250, move || on_next.on_next(42)),
                scheduler.schedule_absolute_action(260, move || observer.on_completed()),
            );
            composite
        })
    };

    let ys = xs.publish();

    let awaiter = awaiter();
    let result = Rc::new(Cell::new(0));
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let awaiter = awaiter.clone();
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(ys.get_awaiter());
        });
    }
    {
        let (scheduler_, awaiter, result, t) = (scheduler.clone(), awaiter.clone(), result.clone(), t.clone());
        scheduler.schedule_absolute_action(200, move || {
            get(&awaiter).continue_with(move |task| {
                t.set(scheduler_.clock());
                result.set(task.result().unwrap());
            });
        });
    }

    scheduler.run();

    assert_eq!(100, s.get());
    assert_eq!(260, t.get());
    assert_eq!(42, result.get());
}

#[test]
fn await_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(20, -1),
        RT::on_next(150, 0),
        RT::on_next(220, 1),
        RT::on_next(290, 2),
        RT::on_next(340, 3),
        RT::on_error(410, ex.clone()),
    ]);

    let awaiter = awaiter();
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let (xs, awaiter) = (xs.clone(), awaiter.clone());
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(xs.as_observable().get_awaiter());
        });
    }
    {
        let (scheduler_, awaiter, t, ex) = (scheduler.clone(), awaiter.clone(), t.clone(), ex.clone());
        scheduler.schedule_absolute_action(200, move || {
            get(&awaiter).continue_with(move |task| {
                t.set(scheduler_.clock());
                assert!(same(&task.result().unwrap_err(), &ex));
            });
        });
    }

    scheduler.run();

    assert_eq!(410, t.get());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe_from(100));
}

#[test]
fn await_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(20, -1),
        RT::on_next(150, 0),
        RT::on_next(220, 1),
        RT::on_next(290, 2),
        RT::on_next(340, 3),
    ]);

    let awaiter = awaiter();
    let has_value = Rc::new(Cell::new(false));
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let (xs, awaiter) = (xs.clone(), awaiter.clone());
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(xs.as_observable().get_awaiter());
        });
    }
    {
        let (scheduler_, awaiter, has_value, t) = (scheduler.clone(), awaiter.clone(), has_value.clone(), t.clone());
        scheduler.schedule_absolute_action(200, move || {
            get(&awaiter).continue_with(move |task| {
                t.set(scheduler_.clock());
                let _ = task.result();
                has_value.set(true);
            });
        });
    }

    scheduler.run();

    assert_eq!(i64::MAX, t.get());
    assert!(!has_value.get());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe_from(100));
}

#[test]
fn await_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(300)]);

    let awaiter = awaiter();
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let (xs, awaiter) = (xs.clone(), awaiter.clone());
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(xs.as_observable().get_awaiter());
        });
    }
    {
        let (scheduler_, awaiter, t) = (scheduler.clone(), awaiter.clone(), t.clone());
        scheduler.schedule_absolute_action(200, move || {
            get(&awaiter).continue_with(move |task| {
                t.set(scheduler_.clock());
                let error = task.result().unwrap_err();
                assert_eq!(Some(&SequenceError::NoElements), error.downcast_ref::<SequenceError>());
            });
        });
    }

    scheduler.run();

    assert_eq!(300, t.get());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe_from(100));
}

// Not ported: Await_ArgumentChecking - argument validation
