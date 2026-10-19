//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ForEachAsyncTest.cs` of Rx.NET.
//! Adapted: the task is a `Task<()>`; the overloads with a token are `_with_token`, those with an
//! index `_with_index`, and an action that throws is the `try_` form returning `Err(ex)`. The
//! blocking `Wait()` is `block_on`. The list the actions fill and the task a scheduled action
//! assigns are shared cells.

use ferroui_reactive::{Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_subscriptions, new_error, ReactiveTest as RT, Recorded, TestScheduler};
use ferroui_system::threading::tasks::{block_on, Task, TaskStatus};
use ferroui_system::threading::CancellationTokenSource;
use std::cell::RefCell;
use std::rc::Rc;

/// The task that a scheduled action assigns (`var task = default(Task);`).
type TaskSlot = Rc<RefCell<Option<Task<()>>>>;

fn get(task: &TaskSlot) -> Task<()> {
    task.borrow().clone().expect("the task has been assigned")
}

/// `Assert.Same` of two errors.
fn same(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

#[test]
fn for_each_async_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
    ]);

    let task: TaskSlot = Rc::new(RefCell::new(None));
    let cts = CancellationTokenSource::new();
    let list: Rc<RefCell<Vec<Recorded<i32>>>> = Rc::new(RefCell::new(Vec::new()));

    {
        let (xs, task, list, clock, token) = (xs.clone(), task.clone(), list.clone(), scheduler.clone(), cts.token());
        scheduler.schedule_absolute_action(150, move || {
            *task.borrow_mut() =
                Some(xs.as_observable().for_each_async_with_token(move |x| list.borrow_mut().push(Recorded::new(clock.clock(), x)), token));
        });
    }

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe_from(150));

    assert_eq!(vec![Recorded::new(200, 2), Recorded::new(300, 3), Recorded::new(400, 4), Recorded::new(500, 5)], *list.borrow());

    assert_eq!(TaskStatus::WaitingForActivation, get(&task).status());
}

#[test]
fn for_each_async_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_completed(600),
    ]);

    let task: TaskSlot = Rc::new(RefCell::new(None));
    let cts = CancellationTokenSource::new();
    let list: Rc<RefCell<Vec<Recorded<i32>>>> = Rc::new(RefCell::new(Vec::new()));

    {
        let (xs, task, list, clock, token) = (xs.clone(), task.clone(), list.clone(), scheduler.clone(), cts.token());
        scheduler.schedule_absolute_action(150, move || {
            *task.borrow_mut() =
                Some(xs.as_observable().for_each_async_with_token(move |x| list.borrow_mut().push(Recorded::new(clock.clock(), x)), token));
        });
    }

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(150, 600));

    assert_eq!(vec![Recorded::new(200, 2), Recorded::new(300, 3), Recorded::new(400, 4), Recorded::new(500, 5)], *list.borrow());

    assert_eq!(TaskStatus::RanToCompletion, get(&task).status());
}

#[test]
fn for_each_async_error() {
    let scheduler = TestScheduler::new();

    let exception = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_error(600, exception.clone()),
    ]);

    let task: TaskSlot = Rc::new(RefCell::new(None));
    let cts = CancellationTokenSource::new();
    let list: Rc<RefCell<Vec<Recorded<i32>>>> = Rc::new(RefCell::new(Vec::new()));

    {
        let (xs, task, list, clock, token) = (xs.clone(), task.clone(), list.clone(), scheduler.clone(), cts.token());
        scheduler.schedule_absolute_action(150, move || {
            *task.borrow_mut() =
                Some(xs.as_observable().for_each_async_with_token(move |x| list.borrow_mut().push(Recorded::new(clock.clock(), x)), token));
        });
    }

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(150, 600));

    assert_eq!(vec![Recorded::new(200, 2), Recorded::new(300, 3), Recorded::new(400, 4), Recorded::new(500, 5)], *list.borrow());

    let task = get(&task);
    assert_eq!(TaskStatus::Faulted, task.status());
    assert!(same(&exception, &task.exception().expect("the task is faulted")));
}

#[test]
fn for_each_async_throw() {
    let scheduler = TestScheduler::new();

    let exception = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_completed(600),
    ]);

    let task: TaskSlot = Rc::new(RefCell::new(None));
    let cts = CancellationTokenSource::new();
    let list: Rc<RefCell<Vec<Recorded<i32>>>> = Rc::new(RefCell::new(Vec::new()));

    {
        let (xs, task, list, clock, token, exception) = (xs.clone(), task.clone(), list.clone(), scheduler.clone(), cts.token(), exception.clone());
        scheduler.schedule_absolute_action(150, move || {
            *task.borrow_mut() = Some(xs.as_observable().try_for_each_async_with_token(
                move |x| {
                    if clock.clock() > 400 {
                        return Err(exception.clone());
                    }

                    list.borrow_mut().push(Recorded::new(clock.clock(), x));
                    Ok(())
                },
                token,
            ));
        });
    }

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(150, 500));

    assert_eq!(vec![Recorded::new(200, 2), Recorded::new(300, 3), Recorded::new(400, 4)], *list.borrow());

    let task = get(&task);
    assert_eq!(TaskStatus::Faulted, task.status());
    assert!(same(&exception, &task.exception().expect("the task is faulted")));
}

#[test]
fn for_each_async_cancel_during() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_completed(600),
    ]);

    let task: TaskSlot = Rc::new(RefCell::new(None));
    let cts = CancellationTokenSource::new();
    let list: Rc<RefCell<Vec<Recorded<i32>>>> = Rc::new(RefCell::new(Vec::new()));

    {
        let (xs, task, list, clock, token) = (xs.clone(), task.clone(), list.clone(), scheduler.clone(), cts.token());
        scheduler.schedule_absolute_action(150, move || {
            *task.borrow_mut() =
                Some(xs.as_observable().for_each_async_with_token(move |x| list.borrow_mut().push(Recorded::new(clock.clock(), x)), token));
        });
    }
    {
        let cts = cts.clone();
        scheduler.schedule_absolute_action(350, move || cts.cancel());
    }

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(150, 350));

    assert_eq!(vec![Recorded::new(200, 2), Recorded::new(300, 3)], *list.borrow());

    assert_eq!(TaskStatus::Canceled, get(&task).status());
}

#[test]
fn for_each_async_cancel_before() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_completed(600),
    ]);

    let task: TaskSlot = Rc::new(RefCell::new(None));
    let cts = CancellationTokenSource::new();
    let list: Rc<RefCell<Vec<Recorded<i32>>>> = Rc::new(RefCell::new(Vec::new()));

    cts.cancel();

    {
        let (xs, task, list, clock, token) = (xs.clone(), task.clone(), list.clone(), scheduler.clone(), cts.token());
        scheduler.schedule_absolute_action(150, move || {
            *task.borrow_mut() =
                Some(xs.as_observable().for_each_async_with_token(move |x| list.borrow_mut().push(Recorded::new(clock.clock(), x)), token));
        });
    }

    scheduler.run();

    assert_subscriptions!(xs.subscriptions());

    assert_eq!(Vec::<Recorded<i32>>::new(), *list.borrow());

    assert_eq!(TaskStatus::Canceled, get(&task).status());
}

#[test]
fn for_each_async_cancel_after() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_completed(600),
    ]);

    let task: TaskSlot = Rc::new(RefCell::new(None));
    let cts = CancellationTokenSource::new();
    let list: Rc<RefCell<Vec<Recorded<i32>>>> = Rc::new(RefCell::new(Vec::new()));

    {
        let (xs, task, list, clock, token) = (xs.clone(), task.clone(), list.clone(), scheduler.clone(), cts.token());
        scheduler.schedule_absolute_action(150, move || {
            *task.borrow_mut() =
                Some(xs.as_observable().for_each_async_with_token(move |x| list.borrow_mut().push(Recorded::new(clock.clock(), x)), token));
        });
    }
    {
        let cts = cts.clone();
        scheduler.schedule_absolute_action(700, move || cts.cancel());
    }

    scheduler.run();

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(150, 600));

    assert_eq!(vec![Recorded::new(200, 2), Recorded::new(300, 3), Recorded::new(400, 4), Recorded::new(500, 5)], *list.borrow());

    assert_eq!(TaskStatus::RanToCompletion, get(&task).status());
}

#[test]
fn for_each_async_default() {
    let list = Rc::new(RefCell::new(Vec::new()));
    let sink = list.clone();
    block_on(Observable::range(1, 10).for_each_async(move |x| sink.borrow_mut().push(x))).unwrap();
    assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], *list.borrow());
}

#[test]
fn for_each_async_index() {
    let list = Rc::new(RefCell::new(Vec::new()));
    let sink = list.clone();
    block_on(Observable::range(3, 5).for_each_async_with_index(move |x, i| sink.borrow_mut().push(x * i as i32))).unwrap();
    // 3 * 0, 4 * 1, 5 * 2, 6 * 3, 7 * 4.
    assert_eq!(vec![0, 4, 10, 18, 28], *list.borrow());
}

// Not ported: ForEachAsync_ArgumentChecking - argument validation
// Not ported: ForEachAsync_Default_Cancel - real threads (Repeat on Scheduler.Default, observed from the test thread)
// Not ported: ForEachAsync_Index_Cancel - real threads (Repeat on Scheduler.Default, observed from the test thread)
// Not ported: ForEachAsync_DisposeThrows1 - a Dispose that throws: disposables do not fail in the port
// Not ported: ForEachAsync_DisposeThrows2 - a Dispose that throws: disposables do not fail in the port
// Not ported: ForEachAsync_DisposeThrows - a Dispose that throws: disposables do not fail in the port; real threads (Scheduler.Default)
// Not ported: ForEachAsync_SubscribeThrows - a subscribe function of Create that throws: the subscription of Observable::create cannot fail
