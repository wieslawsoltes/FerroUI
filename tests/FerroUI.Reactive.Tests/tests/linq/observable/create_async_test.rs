//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/CreateAsyncTest.cs` of Rx.NET.
//! Adapted: `Create` with an asynchronous subscription is `Observable::create_async` (`Task`),
//! `create_async_with_disposable` (`Task<IDisposable>`) and `create_async_with_action`
//! (`Task<Action>`), with `_with_token` for the forms with a `CancellationToken`; the producers
//! return the task of a `TaskCompletionSource<()>` (`TaskCompletionSource<object>`), a producer
//! that throws returns a task that has failed (the same error, at the same time), and
//! `token.Register` is `register_local`. `RunSynchronously` (a synchronous task scheduler) is not
//! needed: the continuations of the tasks run on the thread that ends them. The work that
//! `Task.Factory.StartNew` runs on a worker thread is split: the worker (`tasks::run`) produces
//! the value and the observer is called on the test thread when it arrives, as the observers are
//! not `Send`; the blocking `ForEach` and `Wait()` are `block_on` of `to_list().to_task()` and
//! `to_task()`, and the `ManualResetEvent` that waits for the resource is a flag set when it is
//! released (on the test thread, before `block_on` returns).

use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{IDisposable, IObserver, NotificationKind, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::tasks::{block_on, run, Task, TaskCanceledError, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use std::cell::Cell;
use std::rc::Rc;

/// A producer of a value every 100 ticks, from 300 (`self(TimeSpan.FromTicks(100))`): `step` runs
/// with the count of the values produced before, then the next value is produced; the token
/// disposes the work.
fn produce(
    results: Rc<dyn IObserver<i32>>,
    scheduler: Rc<dyn IScheduler>,
    token: CancellationToken,
    step: impl Fn(i32, &Rc<dyn IObserver<i32>>) + 'static,
) {
    let x = Rc::new(Cell::new(0));

    let d = scheduler.schedule_recursive_after(ticks_to_duration(100), move |recurse| {
        step(x.get(), &results);
        x.set(x.get() + 1);
        results.on_next(x.get());
        recurse(ticks_to_duration(100));
    });

    let _ = token.register_local(move || d.dispose());
}

fn producer1(results: Rc<dyn IObserver<i32>>, scheduler: Rc<dyn IScheduler>, token: CancellationToken) -> Task<()> {
    let tcs = TaskCompletionSource::<()>::new();

    produce(results, scheduler, token, |_, _| {});

    tcs.task()
}

#[test]
fn create_async_never() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::create_async_with_token(move |observer, token| producer1(observer, s.clone(), token)));

    assert_messages!(
        res.messages(),
        RT::on_next(300, 1),
        RT::on_next(400, 2),
        RT::on_next(500, 3),
        RT::on_next(600, 4),
        RT::on_next(700, 5),
        RT::on_next(800, 6),
        RT::on_next(900, 7)
    );
}

fn producer2(results: Rc<dyn IObserver<i32>>, scheduler: Rc<dyn IScheduler>, token: CancellationToken) -> Task<()> {
    let tcs = TaskCompletionSource::<()>::new();

    let source = tcs.clone();
    produce(results, scheduler, token, move |x, _| {
        if x == 4 {
            source.set_result(());
        }
    });

    tcs.task()
}

#[test]
fn create_async_completed1() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::create_async_with_token(move |observer, token| producer2(observer, s.clone(), token)));

    assert_messages!(
        res.messages(),
        RT::on_next(300, 1),
        RT::on_next(400, 2),
        RT::on_next(500, 3),
        RT::on_next(600, 4),
        RT::on_completed(700)
    );
}

fn producer3(results: Rc<dyn IObserver<i32>>, scheduler: Rc<dyn IScheduler>, token: CancellationToken) -> Task<()> {
    let tcs = TaskCompletionSource::<()>::new();

    produce(results, scheduler, token, |x, results| {
        if x == 4 {
            results.on_completed();
        }
    });

    tcs.task()
}

#[test]
fn create_async_completed2() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::create_async_with_token(move |observer, token| producer3(observer, s.clone(), token)));

    assert_messages!(
        res.messages(),
        RT::on_next(300, 1),
        RT::on_next(400, 2),
        RT::on_next(500, 3),
        RT::on_next(600, 4),
        RT::on_completed(700)
    );
}

fn producer4(results: Rc<dyn IObserver<i32>>, scheduler: Rc<dyn IScheduler>, exception: ObservableError, token: CancellationToken) -> Task<()> {
    let tcs = TaskCompletionSource::<()>::new();

    produce(results, scheduler, token, move |x, results| {
        if x == 4 {
            results.on_error(exception.clone());
        }
    });

    tcs.task()
}

#[test]
fn create_async_error1() {
    let scheduler = TestScheduler::new();

    let exception = new_error("ex");

    let s = scheduler.scheduler();
    let ex = exception.clone();
    let res = scheduler.start(move || Observable::create_async_with_token(move |observer, token| producer4(observer, s.clone(), ex.clone(), token)));

    assert_messages!(
        res.messages(),
        RT::on_next(300, 1),
        RT::on_next(400, 2),
        RT::on_next(500, 3),
        RT::on_next(600, 4),
        RT::on_error(700, exception)
    );
}

fn producer5(results: Rc<dyn IObserver<i32>>, scheduler: Rc<dyn IScheduler>, exception: ObservableError, token: CancellationToken) -> Task<()> {
    let tcs = TaskCompletionSource::<()>::new();

    let source = tcs.clone();
    produce(results, scheduler, token, move |x, _| {
        if x == 4 {
            source.set_exception(exception.clone());
        }
    });

    tcs.task()
}

#[test]
fn create_async_error2() {
    let scheduler = TestScheduler::new();

    let exception = new_error("ex");

    let s = scheduler.scheduler();
    let ex = exception.clone();
    let res = scheduler.start(move || Observable::create_async_with_token(move |observer, token| producer5(observer, s.clone(), ex.clone(), token)));

    assert_messages!(
        res.messages(),
        RT::on_next(300, 1),
        RT::on_next(400, 2),
        RT::on_next(500, 3),
        RT::on_next(600, 4),
        RT::on_error(700, exception)
    );
}

/// `throw exception`: a task that has failed with it.
fn producer6(_results: Rc<dyn IObserver<i32>>, exception: ObservableError, _token: CancellationToken) -> Task<()> {
    Task::from_exception(exception)
}

#[test]
fn create_async_error3() {
    let scheduler = TestScheduler::new();

    let exception = new_error("InvalidOperationException");

    let ex = exception.clone();
    let res = scheduler.start(move || Observable::create_async_with_token(move |observer, token| producer6(observer, ex.clone(), token)));

    assert_messages!(res.messages(), RT::on_error::<i32>(200, exception));
}

fn producer7(results: Rc<dyn IObserver<i32>>, scheduler: Rc<dyn IScheduler>, token: CancellationToken) -> Task<()> {
    let tcs = TaskCompletionSource::<()>::new();

    let source = tcs.clone();
    produce(results, scheduler, token, move |x, _| {
        if x == 4 {
            source.set_result(());
        }
    });

    tcs.task()
}

#[test]
fn create_async_cancel1() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::create_async_with_token(move |observer, token| producer7(observer, s.clone(), token)), 650);

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_next(600, 4));
}

fn producer8(results: Rc<dyn IObserver<i32>>, scheduler: Rc<dyn IScheduler>, token: CancellationToken) -> Task<()> {
    let tcs = TaskCompletionSource::<()>::new();

    produce(results, scheduler, token, |x, results| {
        if x == 4 {
            results.on_completed();
        }
    });

    tcs.task()
}

#[test]
fn create_async_cancel2() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::create_async_with_token(move |observer, token| producer8(observer, s.clone(), token)), 650);

    assert_messages!(res.messages(), RT::on_next(300, 1), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_next(600, 4));
}

fn producer9(results: Rc<dyn IObserver<i32>>, scheduler: Rc<dyn IScheduler>, token: CancellationToken) -> Task<()> {
    let tcs = TaskCompletionSource::<()>::new();

    produce(results, scheduler, token, |x, results| {
        if x == 4 {
            results.on_completed();
        }
    });

    tcs.task()
}

#[test]
fn create_async_cancel3() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::create_async_with_token(move |observer, token| producer9(observer, s.clone(), token)), 750);

    assert_messages!(
        res.messages(),
        RT::on_next(300, 1),
        RT::on_next(400, 2),
        RT::on_next(500, 3),
        RT::on_next(600, 4),
        RT::on_completed(700)
    );
}

fn producer10(results: Rc<dyn IObserver<i32>>, scheduler: Rc<dyn IScheduler>, token: CancellationToken) -> Task<()> {
    let tcs = TaskCompletionSource::<()>::new();

    let source = tcs.clone();
    produce(results, scheduler, token, move |x, _| {
        if x == 4 {
            source.set_canceled();
        }
    });

    tcs.task()
}

#[test]
fn create_async_cancel4() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::create_async_with_token(move |observer, token| producer10(observer, s.clone(), token)));

    let messages = res.messages();

    assert_messages!(messages[..4].to_vec(), RT::on_next(300, 1), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_next(600, 4));

    assert_eq!(5, messages.len());

    assert_eq!(700, messages[4].time);
    assert_eq!(NotificationKind::OnError, messages[4].value.kind());
    assert!(messages[4].value.exception().is_some_and(TaskCanceledError::is_cancellation));
}

#[test]
fn create_async_task_simple() {
    let xs = Observable::create_async(|observer: Rc<dyn IObserver<i32>>| async move {
        let value = run(|| Ok(42)).await?;
        observer.on_next(value);
        observer.on_completed();
        Ok::<(), ObservableError>(())
    });

    let lst = block_on(xs.to_list().to_task()).unwrap();

    assert_eq!(vec![42], lst);
}

#[test]
fn create_async_i_disposable_simple() {
    let stopped = Rc::new(Cell::new(false));
    let flag = stopped.clone();
    let s = Disposable::create(move || flag.set(true));

    let xs = Observable::create_async_with_disposable(move |observer: Rc<dyn IObserver<i32>>| {
        let s = s.clone();
        async move {
            let value = run(|| Ok(42)).await?;
            observer.on_next(value);
            observer.on_completed();

            Ok::<Rc<dyn IDisposable>, ObservableError>(s)
        }
    });

    let lst = block_on(xs.to_list().to_task()).unwrap();

    assert!(stopped.get());

    assert_eq!(vec![42], lst);
}

#[test]
fn create_async_action_simple() {
    let stopped = Rc::new(Cell::new(false));

    let flag = stopped.clone();
    let xs = Observable::create_async_with_action(move |observer: Rc<dyn IObserver<i32>>| {
        let flag = flag.clone();
        async move {
            let value = run(|| Ok(42)).await?;
            observer.on_next(value);
            observer.on_completed();

            let s = move || flag.set(true);
            Ok::<_, ObservableError>(s)
        }
    });

    let lst = block_on(xs.to_list().to_task()).unwrap();

    assert!(stopped.get());

    assert_eq!(vec![42], lst);
}

#[test]
fn create_with_task_disposable_no_premature_termination() {
    let obs = Observable::create_async_with_disposable(|o: Rc<dyn IObserver<i32>>| async move {
        // avoid warning on async o due to no await
        std::future::ready(()).await;

        let inner = Observable::range(1, 3);

        Ok::<Rc<dyn IDisposable>, ObservableError>(inner.subscribe_next(move |x| {
            o.on_next(x);
        }))
    });

    let _result = block_on(obs.take(1).to_task()).unwrap();
}

#[test]
fn create_with_task_action_no_premature_termination() {
    let obs = Observable::create_async_with_action(|o: Rc<dyn IObserver<i32>>| async move {
        // avoid warning on async o due to no await
        std::future::ready(()).await;

        let inner = Observable::range(1, 3);

        let d = inner.subscribe_next(move |x| {
            o.on_next(x);
        });

        let a = move || d.dispose();
        Ok::<_, ObservableError>(a)
    });

    let _result = block_on(obs.take(1).to_task()).unwrap();
}

// Not ported: CreateAsync_ArgumentChecking - argument validation
// Not ported: CreateAsync_NullCoalescingAction1 - a null action returned by the task (the future of the port gives an action, which cannot be null)
// Not ported: CreateAsync_NullCoalescingAction2 - a null action returned by the task (the future of the port gives an action, which cannot be null)
// Not ported: CreateAsync_NullCoalescingDisposable1 - a null disposable returned by the task (the future of the port gives an `Rc`, which cannot be null)
// Not ported: CreateAsync_NullCoalescingDisposable2 - a null disposable returned by the task (the future of the port gives an `Rc`, which cannot be null)
// Not ported: CreateAsync_Task_Token - real threads (the work calls the observer on a worker thread until the token is cancelled, coordinated by a ManualResetEvent)
// Not ported: CreateAsync_IDisposable_Token - real threads (the work calls the observer on a worker thread until the token is cancelled, coordinated by a ManualResetEvent)
// Not ported: CreateAsync_Action_Token - real threads (the work calls the observer on a worker thread until the token is cancelled, coordinated by a ManualResetEvent)
