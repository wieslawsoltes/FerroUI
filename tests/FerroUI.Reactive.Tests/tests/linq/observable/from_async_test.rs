//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/FromAsyncTest.cs` of Rx.NET.
//! Adapted: a function returning a `Task` returns a future (a `Task` is one), and the `Task`
//! forms without a result are the same methods with `T = ()`; `Task.Factory.StartNew` is
//! `tasks::run`, and the blocking `Single()` is `block_on` of `single_async().to_task()`. A
//! function that throws synchronously returns a future that fails when it is first polled. An
//! error of work on another thread comes back as a `TaskWorkError` with its message, so the tests
//! of such errors check the message, where upstream checks the identity of the exception. The
//! `ManualResetEvent`s that the worker of the `_Cancel` tests signals are channels, and the spin on
//! `t.IsCompleted` is `block_on(t)`. The `DESKTOPCLR` tests are ported; their wait for the event is
//! not needed, since the continuation runs when the source is completed.

use ferroui_reactive::concurrency::Scheduler;
use ferroui_reactive::{Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::threading::tasks::{block_on, run, Task, TaskCanceledError, TaskCompletionSource, TaskWorkError};
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
    while !token.is_cancellation_requested() {
        std::hint::spin_loop();
    }
    let _ = finished.send(());
    Err(Box::new(TaskCanceledError::default()))
}

#[test]
fn from_async_func_success() {
    let n = 42;

    let i = Rc::new(Cell::new(0));
    let counter = i.clone();
    let xs = Observable::from_async(move || {
        counter.set(counter.get() + 1);
        run(move || Ok(n))
    });

    assert_eq!(n, block_on(xs.single_async().to_task()).unwrap());
    assert_eq!(1, i.get());

    assert_eq!(n, block_on(xs.single_async().to_task()).unwrap());
    assert_eq!(2, i.get());
}

#[test]
fn from_async_func_throw_synchronous() {
    let ex = new_error("ex");

    let error = ex.clone();
    let xs = Observable::from_async(move || std::future::ready(Err::<i32, _>(error.clone())));

    let result = block_on(xs.single_async().to_task());
    assert!(same(&result.unwrap_err(), &ex));
}

#[test]
fn from_async_func_throw_asynchronous() {
    let xs = Observable::from_async(|| run::<i32>(|| Err("ex".into())));

    let result = block_on(xs.single_async().to_task());
    assert_eq!(Some("ex".to_string()), work_error(&result.unwrap_err()));
}

#[test]
fn from_async_func_with_cancel_success() {
    let n = 42;

    let i = Rc::new(Cell::new(0));
    let counter = i.clone();
    let xs = Observable::from_async_with_token(move |_ct| {
        counter.set(counter.get() + 1);
        run(move || Ok(n))
    });

    assert_eq!(n, block_on(xs.single_async().to_task()).unwrap());
    assert_eq!(1, i.get());

    assert_eq!(n, block_on(xs.single_async().to_task()).unwrap());
    assert_eq!(2, i.get());
}

#[test]
fn from_async_func_with_cancel_throw_synchronous() {
    let ex = new_error("ex");

    let error = ex.clone();
    let xs = Observable::from_async_with_token(move |_ct| std::future::ready(Err::<i32, _>(error.clone())));

    let result = block_on(xs.single_async().to_task());
    assert!(same(&result.unwrap_err(), &ex));
}

#[test]
fn from_async_func_with_cancel_throw_asynchronous() {
    let xs = Observable::from_async_with_token(|_ct| run::<i32>(|| Err("ex".into())));

    let result = block_on(xs.single_async().to_task());
    assert_eq!(Some("ex".to_string()), work_error(&result.unwrap_err()));
}

#[test]
fn from_async_func_with_cancel_cancel() {
    let (e, e_wait) = mpsc::channel();
    let (f, f_wait) = mpsc::channel();

    let t: Rc<RefCell<Option<Task<i32>>>> = Rc::new(RefCell::new(None));
    let slot = t.clone();
    let xs = Observable::from_async_with_token(move |ct| {
        let (e, f) = (e.clone(), f.clone());
        let task = run(move || spin_until_canceled::<i32>(ct, e, f));
        *slot.borrow_mut() = Some(task.clone());
        task
    });

    let d = xs.subscribe_next(|_| {});
    e_wait.recv().unwrap();
    d.dispose();

    f_wait.recv().unwrap();
    let t = t.borrow().clone().expect("the function ran");
    let _ = block_on(t.clone());
    assert!(t.is_completed());
}

#[test]
fn from_async_func_scheduler1() {
    let x = Rc::new(Cell::new(0));
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let tcs = TaskCompletionSource::<i32>::new();

    let source = tcs.clone();
    let xs = Observable::from_async_on(move || source.task(), Scheduler::immediate());
    let _d = xs.subscribe_next({
        let (x, t) = (x.clone(), t.clone());
        move |res| {
            x.set(res);
            t.set(Some(thread::current().id()));
        }
    });

    tcs.set_result(42);

    assert_eq!(42, x.get());
    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn from_async_func_scheduler2() {
    let x = Rc::new(Cell::new(0));
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let tcs = TaskCompletionSource::<i32>::new();

    let source = tcs.clone();
    let xs = Observable::from_async_with_token_on(move |_ct| source.task(), Scheduler::immediate());
    let _d = xs.subscribe_next({
        let (x, t) = (x.clone(), t.clone());
        move |res| {
            x.set(res);
            t.set(Some(thread::current().id()));
        }
    });

    tcs.set_result(42);

    assert_eq!(42, x.get());
    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn from_async_action_success() {
    let i = Rc::new(Cell::new(0));
    let counter = i.clone();
    let xs = Observable::from_async(move || {
        counter.set(counter.get() + 1);
        run(|| Ok(()))
    });

    assert!(matches!(block_on(xs.single_async().to_task()), Ok(())));
    assert_eq!(1, i.get());

    assert!(matches!(block_on(xs.single_async().to_task()), Ok(())));
    assert_eq!(2, i.get());
}

#[test]
fn from_async_action_throw_synchronous() {
    let ex = new_error("ex");

    let error = ex.clone();
    let xs = Observable::from_async(move || std::future::ready(Err::<(), _>(error.clone())));

    let result = block_on(xs.single_async().to_task());
    assert!(same(&result.unwrap_err(), &ex));
}

#[test]
fn from_async_action_throw_asynchronous() {
    let xs = Observable::from_async(|| run::<()>(|| Err("ex".into())));

    let result = block_on(xs.single_async().to_task());
    assert_eq!(Some("ex".to_string()), work_error(&result.unwrap_err()));
}

#[test]
fn from_async_action_with_cancel_success() {
    let i = Rc::new(Cell::new(0));
    let counter = i.clone();
    let xs = Observable::from_async_with_token(move |_ct| {
        counter.set(counter.get() + 1);
        // Not forwarding ct because we want this task always to run and complete.
        run(|| Ok(()))
    });

    assert!(matches!(block_on(xs.single_async().to_task()), Ok(())));
    assert_eq!(1, i.get());

    assert!(matches!(block_on(xs.single_async().to_task()), Ok(())));
    assert_eq!(2, i.get());
}

#[test]
fn from_async_action_with_cancel_throw_synchronous() {
    let ex = new_error("ex");

    let error = ex.clone();
    let xs = Observable::from_async_with_token(move |_ct| std::future::ready(Err::<(), _>(error.clone())));

    let result = block_on(xs.single_async().to_task());
    assert!(same(&result.unwrap_err(), &ex));
}

#[test]
fn from_async_action_with_cancel_throw_asynchronous() {
    // Not forwarding ct because we always want this task to run and fail.
    let xs = Observable::from_async_with_token(|_ct| run::<()>(|| Err("ex".into())));

    let result = block_on(xs.single_async().to_task());
    assert_eq!(Some("ex".to_string()), work_error(&result.unwrap_err()));
}

#[test]
fn from_async_action_with_cancel_cancel() {
    let (e, e_wait) = mpsc::channel();
    let (f, f_wait) = mpsc::channel();

    let t: Rc<RefCell<Option<Task<()>>>> = Rc::new(RefCell::new(None));
    let slot = t.clone();
    // The task does not take ct: the test is of a task that is running by the time the
    // cancellation is seen.
    let xs = Observable::from_async_with_token(move |ct| {
        let (e, f) = (e.clone(), f.clone());
        let task = run(move || spin_until_canceled::<()>(ct, e, f));
        *slot.borrow_mut() = Some(task.clone());
        task
    });

    let d = xs.subscribe_next(|_| {});
    e_wait.recv().unwrap();
    d.dispose();

    f_wait.recv().unwrap();
    let t = t.borrow().clone().expect("the function ran");
    let _ = block_on(t.clone());
    assert!(t.is_completed());
}

#[test]
fn from_async_action_scheduler1() {
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let tcs = TaskCompletionSource::<()>::new();

    let source = tcs.clone();
    let xs = Observable::from_async_on(move || source.task(), Scheduler::immediate());
    let _d = xs.subscribe_next({
        let t = t.clone();
        move |_| t.set(Some(thread::current().id()))
    });

    tcs.set_result(());

    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn from_async_action_scheduler2() {
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let tcs = TaskCompletionSource::<()>::new();

    let source = tcs.clone();
    let xs = Observable::from_async_with_token_on(move |_ct| source.task(), Scheduler::immediate());
    let _d = xs.subscribe_next({
        let t = t.clone();
        move |_| t.set(Some(thread::current().id()))
    });

    tcs.set_result(());

    assert_eq!(Some(thread::current().id()), t.get());
}

// Not ported: FromAsync_Func_ArgumentChecking - argument validation
// Not ported: FromAsync_Func_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_FuncWithCancel_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_Func_WithScheduler_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_FuncWithCancel_WithScheduler_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_Func_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_FuncWithCancel_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_Func_WithScheduler_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_FuncWithCancel_WithScheduler_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_Action_ArgumentChecking - argument validation
// Not ported: FromAsync_Action_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_ActionWithCancel_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_Action_WithScheduler_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_ActionWithCancel_WithScheduler_UnsubscribeThenError_ErrorReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_Action_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_ActionWithCancel_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_Action_WithScheduler_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
// Not ported: FromAsync_ActionWithCancel_WithScheduler_IgnorePostUnsubscribeErrors_UnsubscribeThenError_ErrorNotReportedAsUnobserved - unobserved task exceptions (no task scheduler events in the port)
