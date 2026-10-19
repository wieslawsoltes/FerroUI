//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RunAsyncTest.cs` of Rx.NET.
//! Adapted: `RunAsync` gives the task of its `AsyncSubject<T>` (the subject's awaiter), so
//! `awaiter.OnCompleted(continuation)` is `task.continue_with(continuation)` and
//! `awaiter.GetResult()` is `task.result()`; the `OperationCanceledException` of a cancellation is
//! a `TaskCanceledError`. `SynchronizationContext.SetSynchronizationContext(null)` has no
//! counterpart: the continuations of a task run where it ends. The variables that scheduled
//! actions assign are shared cells.

use ferroui_reactive::disposables::{Disposable, StableCompositeDisposable};
use ferroui_reactive::{ConnectableObservableExt, IDisposable, IObservable, IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_subscriptions, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::tasks::{Task, TaskCanceledError};
use ferroui_system::threading::{CancellationToken, CancellationTokenSource};
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

/// `Observable.Create` of the `_Connectable` tests: records the time of the subscription in
/// `subscribed`, notifies 42 at 250 and the end at 260, and records the time of the disposal in
/// `disposed` when there is one.
fn connectable_source(scheduler: &TestScheduler, subscribed: Rc<Cell<Option<i64>>>, disposed: Option<Rc<Cell<i64>>>) -> Rc<dyn IObservable<i32>> {
    let scheduler = scheduler.clone();
    Observable::create(move |observer: Rc<dyn IObserver<i32>>| {
        subscribed.set(Some(scheduler.clock()));

        let on_next = observer.clone();
        let mut disposables = vec![
            scheduler.schedule_absolute_action(250, move || on_next.on_next(42)),
            scheduler.schedule_absolute_action(260, move || observer.on_completed()),
        ];
        if let Some(disposed) = disposed.clone() {
            let clock = scheduler.clone();
            disposables.push(Disposable::create(move || disposed.set(clock.clock())));
        }
        let composite: Rc<dyn IDisposable> = StableCompositeDisposable::create_from(disposables);
        composite
    })
}

#[test]
fn run_async_simple() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(220, 42),
        RT::on_completed::<i32>(250),
    ]);

    let awaiter = awaiter();
    let result = Rc::new(Cell::new(0));
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let (xs, awaiter) = (xs.clone(), awaiter.clone());
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(xs.as_observable().run_async(CancellationToken::none()));
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

    assert_eq!(250, t.get());
    assert_eq!(42, result.get());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe_from(100));
}

#[test]
fn run_async_cancelled() {
    let cts = CancellationTokenSource::new();
    cts.cancel();

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(220, 42),
        RT::on_completed::<i32>(250),
    ]);

    let awaiter = awaiter();
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let (xs, awaiter, token) = (xs.clone(), awaiter.clone(), cts.token());
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(xs.as_observable().run_async(token));
        });
    }
    {
        let (scheduler_, awaiter, t) = (scheduler.clone(), awaiter.clone(), t.clone());
        scheduler.schedule_absolute_action(200, move || {
            get(&awaiter).continue_with(move |task| {
                t.set(scheduler_.clock());

                assert!(TaskCanceledError::is_cancellation(&task.result().unwrap_err()));
            });
        });
    }

    scheduler.run();

    assert_eq!(200, t.get());

    assert_subscriptions!(xs.subscriptions());
}

#[test]
fn run_async_cancel() {
    let cts = CancellationTokenSource::new();

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(220, 42),
        RT::on_completed::<i32>(250),
    ]);

    let awaiter = awaiter();
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let (xs, awaiter, token) = (xs.clone(), awaiter.clone(), cts.token());
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(xs.as_observable().run_async(token));
        });
    }
    {
        let (scheduler_, awaiter, t) = (scheduler.clone(), awaiter.clone(), t.clone());
        scheduler.schedule_absolute_action(200, move || {
            get(&awaiter).continue_with(move |task| {
                t.set(scheduler_.clock());

                assert!(TaskCanceledError::is_cancellation(&task.result().unwrap_err()));
            });
        });
    }
    {
        let cts = cts.clone();
        scheduler.schedule_absolute_action(210, move || cts.cancel());
    }

    scheduler.run();

    assert_eq!(210, t.get());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(100, 210));
}

#[test]
fn run_async_connectable() {
    let scheduler = TestScheduler::new();

    let s = Rc::new(Cell::new(None));

    let xs = connectable_source(&scheduler, s.clone(), None);

    let ys = xs.publish();

    let awaiter = awaiter();
    let result = Rc::new(Cell::new(0));
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let awaiter = awaiter.clone();
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(ys.run_async(CancellationToken::none()));
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

    assert_eq!(Some(100), s.get());
    assert_eq!(260, t.get());
    assert_eq!(42, result.get());
}

#[test]
fn run_async_connectable_cancelled() {
    let cts = CancellationTokenSource::new();
    cts.cancel();

    let scheduler = TestScheduler::new();

    let s = Rc::new(Cell::new(None));

    let xs = connectable_source(&scheduler, s.clone(), None);

    let ys = xs.publish();

    let awaiter = awaiter();
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let (awaiter, token) = (awaiter.clone(), cts.token());
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(ys.run_async(token));
        });
    }
    {
        let (scheduler_, awaiter, t) = (scheduler.clone(), awaiter.clone(), t.clone());
        scheduler.schedule_absolute_action(200, move || {
            get(&awaiter).continue_with(move |task| {
                t.set(scheduler_.clock());

                assert!(TaskCanceledError::is_cancellation(&task.result().unwrap_err()));
            });
        });
    }

    scheduler.run();

    assert!(s.get().is_none());
    assert_eq!(200, t.get());
}

#[test]
fn run_async_connectable_cancel() {
    let cts = CancellationTokenSource::new();

    let scheduler = TestScheduler::new();

    let s = Rc::new(Cell::new(None));
    let d = Rc::new(Cell::new(0));

    let xs = connectable_source(&scheduler, s.clone(), Some(d.clone()));

    let ys = xs.publish();

    let awaiter = awaiter();
    let t = Rc::new(Cell::new(i64::MAX));

    {
        let (awaiter, token) = (awaiter.clone(), cts.token());
        scheduler.schedule_absolute_action(100, move || {
            *awaiter.borrow_mut() = Some(ys.run_async(token));
        });
    }
    {
        let (scheduler_, awaiter, t) = (scheduler.clone(), awaiter.clone(), t.clone());
        scheduler.schedule_absolute_action(200, move || {
            get(&awaiter).continue_with(move |task| {
                t.set(scheduler_.clock());

                assert!(TaskCanceledError::is_cancellation(&task.result().unwrap_err()));
            });
        });
    }
    {
        let cts = cts.clone();
        scheduler.schedule_absolute_action(210, move || cts.cancel());
    }

    scheduler.run();

    assert_eq!(Some(100), s.get());
    assert_eq!(210, d.get());
    assert_eq!(210, t.get());
}

// Not ported: RunAsync_ArgumentChecking - argument validation
