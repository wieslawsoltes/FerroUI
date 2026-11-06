//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/StartAsyncTest.cs` of Rx.NET.
//! Adapted: a function returning a `Task` returns a future of its result, and the `Task` forms
//! without a result are the same methods with `T = ()`; `Task.Factory.StartNew` is `tasks::run`,
//! whose `Shared` gives the future with `flatten()`, a `TaskCompletionSource` is a `oneshot`
//! channel (its task the `flatten()` of the receiver), and the blocking `Single()` is `block_on`
//! of `single_async().to_task().flatten()`. A
//! function that throws synchronously returns a future that fails when it is first polled. An
//! error of work on another thread comes back as a `TaskWorkError` with its message, so the tests
//! of such errors check the message, where upstream checks the identity of the exception; the
//! `OperationCanceledException` the worker of the `_Cancel` tests throws is such an error with the
//! message of a cancellation (`Canceled`). The `ManualResetEvent`s that the worker signals are channels,
//! and the spin on `t.IsCompleted` is `block_on(t.flatten())`. The `DESKTOPCLR` tests are ported; their wait
//! for the event is not needed, since the continuation runs when the source is completed.

use ferroui_reactive::concurrency::Scheduler;
use ferroui_reactive::{Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::task::{block_on, oneshot, Canceled, Shared};
use ferroui_system::threading::tasks::{run, TaskWorkError};
use ferroui_system::threading::CancellationToken;
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, ThreadId};

/// `Assert.Same` of two errors.
fn same(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// The message of an error of work on another thread (`ReactiveAssert.Throws(ex, ...)` of an
/// exception thrown by `Task.Factory.StartNew`).
fn work_error(error: &ObservableError) -> Option<String> {
    error.downcast_ref::<TaskWorkError>().map(|error| error.message.clone())
}

/// The work of the `_Cancel` tests: signals `started` (`e.Set()`), spins until `token` is
/// canceled (`ct.ThrowIfCancellationRequested()`), and signals `finished` on the way out
/// (`finally { f.Set(); }`).
fn spin_until_canceled<T>(token: CancellationToken, started: Sender<()>, finished: Sender<()>) -> Result<T, Box<dyn Error + Send + Sync>> {
    let _ = started.send(());
    while !token.is_cancelled() {
        std::hint::spin_loop();
    }
    let _ = finished.send(());
    Err(Box::new(Canceled))
}

#[test]
fn start_async_func_success() {
    let n = 42;

    let i = Rc::new(Cell::new(0));

    let counter = i.clone();
    let xs = Observable::start_async(move || {
        counter.set(counter.get() + 1);
        run(move || Ok(n)).flatten()
    });

    assert_eq!(n, block_on(xs.single_async().to_task().flatten()).unwrap());
    assert_eq!(1, i.get());

    assert_eq!(n, block_on(xs.single_async().to_task().flatten()).unwrap());
    assert_eq!(1, i.get());
}

#[test]
fn start_async_func_throw_synchronous() {
    let ex = new_error("ex");

    let error = ex.clone();
    let xs = Observable::start_async(move || std::future::ready(Err::<i32, _>(error)));

    let result = block_on(xs.single_async().to_task().flatten());
    assert!(same(&result.unwrap_err(), &ex));
}

#[test]
fn start_async_func_throw_asynchronous() {
    let xs = Observable::start_async(|| run::<i32>(|| Err("ex".into())).flatten());

    let result = block_on(xs.single_async().to_task().flatten());
    assert_eq!(Some("ex".to_string()), work_error(&result.unwrap_err()));
}

#[test]
fn start_async_func_with_cancel_success() {
    let n = 42;

    let i = Rc::new(Cell::new(0));
    let counter = i.clone();
    let xs = Observable::start_async_with_token(move |_ct| {
        counter.set(counter.get() + 1);
        run(move || Ok(n)).flatten()
    });

    assert_eq!(n, block_on(xs.single_async().to_task().flatten()).unwrap());
    assert_eq!(1, i.get());

    assert_eq!(n, block_on(xs.single_async().to_task().flatten()).unwrap());
    assert_eq!(1, i.get());
}

#[test]
fn start_async_func_with_cancel_throw_synchronous() {
    let ex = new_error("ex");

    let error = ex.clone();
    let xs = Observable::start_async_with_token(move |_ct| std::future::ready(Err::<i32, _>(error)));

    let result = block_on(xs.single_async().to_task().flatten());
    assert!(same(&result.unwrap_err(), &ex));
}

#[test]
fn start_async_func_with_cancel_throw_asynchronous() {
    let xs = Observable::start_async_with_token(|_ct| run::<i32>(|| Err("ex".into())).flatten());

    let result = block_on(xs.single_async().to_task().flatten());
    assert_eq!(Some("ex".to_string()), work_error(&result.unwrap_err()));
}

#[test]
fn start_async_func_with_cancel_cancel() {
    let n_count = 10;

    for _ in 0..n_count {
        let (e, e_wait) = mpsc::channel();
        let (f, f_wait) = mpsc::channel();

        let t: Rc<RefCell<Option<Shared<Result<i32, ObservableError>>>>> = Rc::new(RefCell::new(None));
        let slot = t.clone();
        let xs = Observable::start_async_with_token(move |ct| {
            let task = run(move || spin_until_canceled::<i32>(ct, e, f));
            *slot.borrow_mut() = Some(task.clone());
            task.flatten()
        });

        e_wait.recv().unwrap();

        let d = xs.subscribe_next(|_| {});
        d.dispose();

        f_wait.recv().unwrap();
        let t = t.borrow().clone().expect("the function ran");
        let _ = block_on(t.clone().flatten());
        assert!(t.is_finished());

        let result = block_on(xs.single_async().to_task().flatten());
        assert_eq!(Some(Canceled.to_string()), work_error(&result.unwrap_err()));
    }
}

#[test]
fn start_async_func_scheduler1() {
    let (tcs, source) = oneshot::channel::<Result<i32, ObservableError>>();

    let x = Rc::new(Cell::new(0));
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let xs = Observable::start_async_on(move || source.flatten(), Scheduler::immediate());
    let _d = xs.subscribe_next({
        let (x, t) = (x.clone(), t.clone());
        move |res| {
            x.set(res);
            t.set(Some(thread::current().id()));
        }
    });

    tcs.send(Ok(42)).unwrap();

    assert_eq!(42, x.get());
    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn start_async_func_scheduler2() {
    let (tcs, source) = oneshot::channel::<Result<i32, ObservableError>>();

    let x = Rc::new(Cell::new(0));
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let xs = Observable::start_async_with_token_on(move |_ct| source.flatten(), Scheduler::immediate());
    let _d = xs.subscribe_next({
        let (x, t) = (x.clone(), t.clone());
        move |res| {
            x.set(res);
            t.set(Some(thread::current().id()));
        }
    });

    tcs.send(Ok(42)).unwrap();

    assert_eq!(42, x.get());
    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn start_async_action_success() {
    let i = Rc::new(Cell::new(0));
    let counter = i.clone();
    let xs = Observable::start_async(move || {
        counter.set(counter.get() + 1);
        run(|| Ok(())).flatten()
    });

    assert!(matches!(block_on(xs.single_async().to_task().flatten()), Ok(())));
    assert_eq!(1, i.get());

    assert!(matches!(block_on(xs.single_async().to_task().flatten()), Ok(())));
    assert_eq!(1, i.get());
}

#[test]
fn start_async_action_throw_synchronous() {
    let ex = new_error("ex");

    let error = ex.clone();
    let xs = Observable::start_async(move || std::future::ready(Err::<(), _>(error)));

    let result = block_on(xs.single_async().to_task().flatten());
    assert!(same(&result.unwrap_err(), &ex));
}

#[test]
fn start_async_action_throw_asynchronous() {
    let xs = Observable::start_async(|| run::<()>(|| Err("ex".into())).flatten());

    let result = block_on(xs.single_async().to_task().flatten());
    assert_eq!(Some("ex".to_string()), work_error(&result.unwrap_err()));
}

#[test]
fn start_async_action_with_cancel_success() {
    let i = Rc::new(Cell::new(0));
    let counter = i.clone();
    let xs = Observable::start_async_with_token(move |_ct| {
        counter.set(counter.get() + 1);
        // Not forwarding ct because we always want this task to run to completion in this test.
        run(|| Ok(())).flatten()
    });

    assert!(matches!(block_on(xs.single_async().to_task().flatten()), Ok(())));
    assert_eq!(1, i.get());

    assert!(matches!(block_on(xs.single_async().to_task().flatten()), Ok(())));
    assert_eq!(1, i.get());
}

#[test]
fn start_async_action_with_cancel_throw_synchronous() {
    let ex = new_error("ex");

    let error = ex.clone();
    let xs = Observable::start_async_with_token(move |_ct| std::future::ready(Err::<(), _>(error)));

    let result = block_on(xs.single_async().to_task().flatten());
    assert!(same(&result.unwrap_err(), &ex));
}

#[test]
fn start_async_action_with_cancel_throw_asynchronous() {
    // Not forwarding ct because we always want this task to run and then fail in this test.
    let xs = Observable::start_async_with_token(|_ct| run::<()>(|| Err("ex".into())).flatten());

    let result = block_on(xs.single_async().to_task().flatten());
    assert_eq!(Some("ex".to_string()), work_error(&result.unwrap_err()));
}

#[test]
fn start_async_action_with_cancel_cancel() {
    let n_count = 10;

    for _ in 0..n_count {
        let (e, e_wait) = mpsc::channel();
        let (f, f_wait) = mpsc::channel();

        let t: Rc<RefCell<Option<Shared<Result<(), ObservableError>>>>> = Rc::new(RefCell::new(None));
        let slot = t.clone();
        // The task does not take ct: the test is of a task that is running by the time the
        // cancellation is seen.
        let xs = Observable::start_async_with_token(move |ct| {
            let task = run(move || spin_until_canceled::<()>(ct, e, f));
            *slot.borrow_mut() = Some(task.clone());
            task.flatten()
        });

        e_wait.recv().unwrap();

        let d = xs.subscribe_next(|_| {});
        d.dispose();

        f_wait.recv().unwrap();
        let t = t.borrow().clone().expect("the function ran");
        let _ = block_on(t.clone().flatten());
        assert!(t.is_finished());

        let result = block_on(xs.single_async().to_task().flatten());
        assert_eq!(Some(Canceled.to_string()), work_error(&result.unwrap_err()));
    }
}

#[test]
fn start_async_action_scheduler1() {
    let (tcs, source) = oneshot::channel::<Result<(), ObservableError>>();

    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let xs = Observable::start_async_on(move || source.flatten(), Scheduler::immediate());
    let _d = xs.subscribe_next({
        let t = t.clone();
        move |_| t.set(Some(thread::current().id()))
    });

    tcs.send(Ok(())).unwrap();

    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn start_async_action_scheduler2() {
    let (tcs, source) = oneshot::channel::<Result<(), ObservableError>>();

    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let xs = Observable::start_async_with_token_on(move |_ct| source.flatten(), Scheduler::immediate());
    let _d = xs.subscribe_next({
        let t = t.clone();
        move |_| t.set(Some(thread::current().id()))
    });

    tcs.send(Ok(())).unwrap();

    assert_eq!(Some(thread::current().id()), t.get());
}

// Not ported: StartAsync_Func_ArgumentChecking - argument validation
// Not ported: Start_Func_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_FuncWithCancel_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_Func_WithScheduler_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_FuncWithCancel_WithScheduler_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_Func_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_FuncWithCancel_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_Func_WithScheduler_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_FuncWithCancel_WithScheduler_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: StartAsync_Action_ArgumentChecking - argument validation
// Not ported: Start_Action_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_ActionWithCancel_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_Action_WithScheduler_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_ActionWithCancel_WithScheduler_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_Action_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_ActionWithCancel_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_Action_WithScheduler_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: Start_ActionWithCancel_WithScheduler_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
