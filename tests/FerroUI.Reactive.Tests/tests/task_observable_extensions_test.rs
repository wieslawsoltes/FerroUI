//! Ported from `Tests.System.Reactive/Tests/TaskObservableExtensionsTest.cs` of Rx.NET.
//! Adapted: upstream runs the body of the `TaskToObservable_*` tests through a `TaskFactory` of a
//! `TestTaskScheduler`, which runs it inline; here the body runs directly, through
//! `observe_task`. A `TaskCompletionSource<T>` is a `oneshot` channel of a
//! `Result<T, ObservableError>` (`SetResult` is `send(Ok)`, `SetException` is `send(Err)`,
//! `SetCanceled` is dropping the sender), and its task the `flatten()` of the receiver; upstream's
//! observing continuation (`ContinueWith(t => { var e = t.Exception; })`) has no counterpart on a
//! channel and is left out. The `Task` forms without a result (`((Task)taskSource.Task).ToObservable()`)
//! are the same methods with `T = ()`, on a channel of `Result<(), ObservableError>`.
//! `InvalidOperationException` of a sequence without values is `SequenceError::NoElements`, and the
//! errors of the tests are checked by identity. The `DESKTOPCLR` tests are ported; their wait for
//! the event is not needed, since the continuation runs when the source is completed. The task of
//! `ToTask` is the `Shared` of its result: `IsCompleted` is `is_finished`, `IsFaulted` an error that
//! is not a cancellation, `IsCanceled` the error `Canceled`; the cancellation error of the port
//! carries no token, so `new TaskCanceledException(task).CancellationToken` is not compared.

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction, Scheduler};
use ferroui_reactive::linq::SequenceError;
use ferroui_reactive::{IDisposable, Observable, ObservableError, ObservableExt, TaskObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, MockObserver, ReactiveTest as RT, TestScheduler};
use ferroui_system::task::{oneshot, Canceled, Shared};
use ferroui_system::threading::CancellationToken;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::thread::{self, ThreadId};
use std::time::{Duration, SystemTime};

/// `Assert.Same` of two errors.
fn same(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// The sender of a channel that plays a `TaskCompletionSource<T>`.
type TaskSource<T> = oneshot::Sender<Result<T, ObservableError>>;

/// The body of the `TaskToObservable_*` tests: a channel whose sender `end` ends at `time` (the
/// completion source), and the observer of `taskSource.Task.ToObservable()` (of the receiver)
/// created at 100, subscribed at 200 and disposed at 1000.
fn observe_task<T: Clone + 'static>(time: i64, end: impl FnOnce(TaskSource<T>) + 'static) -> Rc<MockObserver<T>> {
    let scheduler = TestScheduler::new();

    let (task_source, task) = oneshot::channel::<Result<T, ObservableError>>();

    scheduler.schedule_absolute_action(time, move || end(task_source));

    scheduler.start(move || task.flatten().to_observable())
}

/// `SetResult` (a value nobody receives any more is dropped, as the result of a task nobody
/// observes).
fn set_result<T>(task_source: TaskSource<T>, value: T) {
    let _ = task_source.send(Ok(value));
}

/// `SetException`.
fn set_exception<T>(task_source: TaskSource<T>, error: ObservableError) {
    let _ = task_source.send(Err(error));
}

/// `SetCanceled`: the sender dropped without a value.
fn set_canceled<T>(task_source: TaskSource<T>) {
    drop(task_source);
}

/// `IsCompleted` of the task of `ToTask`.
fn is_completed<T: Clone + 'static>(task: &Shared<Result<T, ObservableError>>) -> bool {
    task.is_finished()
}

/// `IsFaulted`: ended with an error that is not a cancellation.
fn is_faulted<T: Clone + 'static>(task: &Shared<Result<T, ObservableError>>) -> bool {
    matches!(task.result(), Some(Err(ref error)) if !Canceled::is_cancellation(error))
}

/// `IsCanceled`: ended with a cancellation.
fn is_canceled<T: Clone + 'static>(task: &Shared<Result<T, ObservableError>>) -> bool {
    matches!(task.result(), Some(Err(ref error)) if Canceled::is_cancellation(error))
}

/// `Exception` of a faulted task.
fn exception<T: Clone + 'static>(task: &Shared<Result<T, ObservableError>>) -> Option<ObservableError> {
    match task.result() {
        Some(Err(error)) if !Canceled::is_cancellation(&error) => Some(error),
        _ => None,
    }
}

#[test]
fn task_to_observable_non_void_complete_before_create() {
    let res = observe_task::<i32>(10, |task_source| set_result(task_source, 42));

    assert_messages!(res.messages(), RT::on_next(200, 42), RT::on_completed::<i32>(200));
}

#[test]
fn task_to_observable_non_void_complete_before_subscribe() {
    let res = observe_task::<i32>(110, |task_source| set_result(task_source, 42));

    assert_messages!(res.messages(), RT::on_next(200, 42), RT::on_completed::<i32>(200));
}

#[test]
fn task_to_observable_non_void_complete_before_dispose() {
    let res = observe_task::<i32>(300, |task_source| set_result(task_source, 42));

    assert_messages!(res.messages(), RT::on_next(300, 42), RT::on_completed::<i32>(300));
}

#[test]
fn task_to_observable_non_void_complete_after_dispose() {
    let res = observe_task::<i32>(1100, |task_source| set_result(task_source, 42));

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_non_void_exception_before_create() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<i32>(10, move |task_source| set_exception(task_source, error));

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));
}

#[test]
fn task_to_observable_non_void_exception_before_subscribe() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<i32>(110, move |task_source| set_exception(task_source, error));

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));
}

#[test]
fn task_to_observable_non_void_exception_before_dispose() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<i32>(300, move |task_source| set_exception(task_source, error));

    assert_messages!(res.messages(), RT::on_error::<i32>(300, ex));
}

#[test]
fn task_to_observable_non_void_exception_after_dispose() {
    let res = observe_task::<i32>(1100, |task_source| set_exception(task_source, new_error("ex")));

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_non_void_canceled_before_create() {
    let res = observe_task::<i32>(10, set_canceled);

    assert_messages!(res.messages(), RT::on_error_where::<i32>(200, Canceled::is_cancellation));
}

#[test]
fn task_to_observable_non_void_canceled_before_subscribe() {
    let res = observe_task::<i32>(110, set_canceled);

    assert_messages!(res.messages(), RT::on_error_where::<i32>(200, Canceled::is_cancellation));
}

#[test]
fn task_to_observable_non_void_canceled_before_dispose() {
    let res = observe_task::<i32>(300, set_canceled);

    assert_messages!(res.messages(), RT::on_error_where::<i32>(300, Canceled::is_cancellation));
}

#[test]
fn task_to_observable_non_void_canceled_after_dispose() {
    let res = observe_task::<i32>(1100, set_canceled);

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_non_void_scheduler() {
    let x = Rc::new(Cell::new(0));
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let (cts, task) = oneshot::channel::<Result<i32, ObservableError>>();

    let xs = task.flatten().to_observable_on(Scheduler::immediate());
    let _d = xs.subscribe_next({
        let (x, t) = (x.clone(), t.clone());
        move |res| {
            x.set(res);
            t.set(Some(thread::current().id()));
        }
    });

    set_result(cts, 42);

    assert_eq!(42, x.get());
    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn task_to_observable_void_complete_before_create() {
    let res = observe_task::<()>(10, |task_source| set_result(task_source, ()));

    assert_messages!(res.messages(), RT::on_next(200, ()), RT::on_completed::<()>(200));
}

#[test]
fn task_to_observable_void_complete_before_subscribe() {
    let res = observe_task::<()>(110, |task_source| set_result(task_source, ()));

    assert_messages!(res.messages(), RT::on_next(200, ()), RT::on_completed::<()>(200));
}

#[test]
fn task_to_observable_void_complete_before_dispose() {
    let res = observe_task::<()>(300, |task_source| set_result(task_source, ()));

    assert_messages!(res.messages(), RT::on_next(300, ()), RT::on_completed::<()>(300));
}

#[test]
fn task_to_observable_void_complete_after_dispose() {
    let res = observe_task::<()>(1100, |task_source| set_result(task_source, ()));

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_void_exception_before_create() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<()>(10, move |task_source| set_exception(task_source, error));

    assert_messages!(res.messages(), RT::on_error::<()>(200, ex));
}

#[test]
fn task_to_observable_void_exception_before_subscribe() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<()>(110, move |task_source| set_exception(task_source, error));

    assert_messages!(res.messages(), RT::on_error::<()>(200, ex));
}

#[test]
fn task_to_observable_void_exception_before_dispose() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<()>(300, move |task_source| set_exception(task_source, error));

    assert_messages!(res.messages(), RT::on_error::<()>(300, ex));
}

#[test]
fn task_to_observable_void_exception_after_dispose() {
    let res = observe_task::<()>(1100, |task_source| set_exception(task_source, new_error("ex")));

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_void_canceled_before_create() {
    let res = observe_task::<()>(10, set_canceled);

    assert_messages!(res.messages(), RT::on_error_where::<()>(200, Canceled::is_cancellation));
}

#[test]
fn task_to_observable_void_canceled_before_subscribe() {
    let res = observe_task::<()>(110, set_canceled);

    assert_messages!(res.messages(), RT::on_error_where::<()>(200, Canceled::is_cancellation));
}

#[test]
fn task_to_observable_void_canceled_before_dispose() {
    let res = observe_task::<()>(300, set_canceled);

    assert_messages!(res.messages(), RT::on_error_where::<()>(300, Canceled::is_cancellation));
}

#[test]
fn task_to_observable_void_canceled_after_dispose() {
    let res = observe_task::<()>(1100, set_canceled);

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_void_scheduler() {
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let (tcs, task) = oneshot::channel::<Result<(), ObservableError>>();

    let xs = task.flatten().to_observable_on(Scheduler::immediate());
    let _d = xs.subscribe_next({
        let t = t.clone();
        move |_| t.set(Some(thread::current().id()))
    });

    set_result(tcs, ());

    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn observable_to_task_no_value() {
    let scheduler = TestScheduler::new();

    let xs = Observable::empty_on::<i32>(scheduler.scheduler());

    let continuation = xs.to_task();
    scheduler.run();

    assert!(is_faulted(&continuation));
    let error = exception(&continuation).expect("the task is faulted");
    assert_eq!(Some(&SequenceError::NoElements), error.downcast_ref::<SequenceError>());
    assert_eq!(1, scheduler.clock());
}

#[test]
fn observable_to_task_single_value() {
    let scheduler = TestScheduler::new();

    let xs = Observable::return_on(5, scheduler.scheduler());

    let continuation = xs.to_task();
    scheduler.run();

    assert!(is_completed(&continuation));
    assert_eq!(5, continuation.result().unwrap().unwrap());
    assert_eq!(1, scheduler.clock());
}

#[test]
fn observable_to_task_multiple_values() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(150, 2),
        RT::on_next(200, 3),
        RT::on_completed(200),
    ]);

    let continuation = xs.as_observable().to_task();
    scheduler.run();

    assert!(is_completed(&continuation));
    assert_eq!(3, continuation.result().unwrap().unwrap());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(0, 200));
}

#[test]
fn observable_to_task_exception() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(150, 2),
        RT::on_error(200, ex.clone()),
    ]);

    let continuation = xs.as_observable().to_task();
    scheduler.run();

    assert!(is_faulted(&continuation));
    let ag = exception(&continuation);
    assert!(ag.is_some());
    assert!(same(&ex, &ag.unwrap()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(0, 200));
}

#[test]
fn observable_to_task_cancelled() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(150, 2),
        RT::on_completed(200),
    ]);

    let cts = CancellationToken::new();

    let continuation = xs.as_observable().to_task_with_token(cts.clone());
    {
        let cts = cts.clone();
        scheduler.schedule_absolute_action(125, move || cts.cancel());
    }
    scheduler.run();

    assert!(is_canceled(&continuation));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(0, 125));
}

#[test]
fn to_task_scheduler_resumed_on_thread_success() {
    let scheduler = OneshotScheduler::new();

    let task = Observable::return_(1).to_task_on(scheduler.clone());

    assert!(scheduler.has_task());

    scheduler.run();

    assert!(!scheduler.has_task());

    assert!(is_completed(&task));
}

#[test]
fn to_task_scheduler_resumed_on_thread_failure() {
    let scheduler = OneshotScheduler::new();

    let ex = new_error("failure");
    let task = Observable::throw::<i32>(ex.clone()).to_task_on(scheduler.clone());

    assert!(scheduler.has_task());

    scheduler.run();

    assert!(!scheduler.has_task());

    assert!(is_faulted(&task));
    let error = exception(&task).expect("the task is faulted");
    assert!(same(&ex, &error));
    assert_eq!("failure", error.to_string());
}

#[test]
fn to_task_scheduler_resumed_on_thread_success_with_cancellation() {
    let scheduler = OneshotScheduler::new();

    let task = Observable::return_(1).to_task_with_token_on(Default::default(), scheduler.clone());

    assert!(scheduler.has_task());

    scheduler.run();

    assert!(!scheduler.has_task());

    assert!(is_completed(&task));
}

#[test]
fn to_task_scheduler_resumed_on_thread_failure_with_cancellation() {
    let scheduler = OneshotScheduler::new();

    let ex = new_error("failure");
    let task = Observable::throw::<i32>(ex.clone()).to_task_with_token_on(Default::default(), scheduler.clone());

    assert!(scheduler.has_task());

    scheduler.run();

    assert!(!scheduler.has_task());

    assert!(is_faulted(&task));
    let error = exception(&task).expect("the task is faulted");
    assert!(same(&ex, &error));
    assert_eq!("failure", error.to_string());
}

#[test]
fn to_task_scheduler_resumed_on_thread_cancel_with_cancellation() {
    let scheduler = OneshotScheduler::new();

    let cts = CancellationToken::new();

    let task = Observable::never::<i32>().to_task_with_token_on(cts.clone(), scheduler.clone());

    assert!(!scheduler.has_task());

    cts.cancel();

    assert!(scheduler.has_task());

    scheduler.run();

    assert!(!scheduler.has_task());

    assert!(is_canceled(&task));
}

/// A scheduler that keeps the last action scheduled and runs it on demand (`OneshotScheduler`).
struct OneshotScheduler {
    this: Weak<OneshotScheduler>,
    task: RefCell<Option<Box<dyn FnOnce()>>>,
}

/// A scheduled action, which disposing forgets (`Work<TState>`).
struct Work {
    action: RefCell<Option<ScheduledAction>>,
}

impl IDisposable for Work {
    fn dispose(&self) {
        let action = self.action.borrow_mut().take();
        drop(action);
    }
}

impl OneshotScheduler {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), task: RefCell::new(None) })
    }

    /// Runs the action scheduled last, once (`Run`).
    fn run(&self) {
        let task = self.task.borrow_mut().take();
        if let Some(task) = task {
            task();
        }
    }

    /// Whether an action waits (`HasTask`).
    fn has_task(&self) -> bool {
        self.task.borrow().is_some()
    }
}

impl IScheduler for OneshotScheduler {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        let work = Rc::new(Work { action: RefCell::new(Some(action)) });
        let (item, this) = (work.clone(), self.this.clone());
        *self.task.borrow_mut() = Some(Box::new(move || {
            let action = item.action.borrow_mut().take();
            if let (Some(action), Some(this)) = (action, this.upgrade()) {
                let scheduler: Rc<dyn IScheduler> = this;
                action(&scheduler);
            }
        }));
        work
    }

    fn schedule_after(&self, _due_time: Duration, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        panic!("Not supported by this scheduler");
    }

    fn schedule_at(&self, _due_time: SystemTime, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        panic!("Not supported by this scheduler");
    }
}

// Not ported: TaskToObservable_NonVoid_ArgumentChecking - argument validation
// Not ported: TaskToObservable_Void_ArgumentChecking - argument validation
// Not ported: ObservableToTask_ArgumentChecking - argument validation
// Not ported: ObservableToTaskWithStateSingleValue - ToTask with a state object (AsyncState): the tasks of the port have no state
// Not ported: ToTask_Scheduler_Resumed_On_Thread_Cancel - calls ContinueOnScheduler, internal to the crate (pub(crate))
// Not ported: ToTask_Scheduler_Resumed_On_Thread_Success_With_State - ToTask with a state object (AsyncState): the tasks of the port have no state
// Not ported: ToTask_Scheduler_Resumed_On_Thread_Failure_With_State - ToTask with a state object (AsyncState): the tasks of the port have no state
// Not ported: ToTask_Scheduler_Resumed_On_Thread_Cancel_With_State - ToTask with a state object (AsyncState): the tasks of the port have no state; calls ContinueOnScheduler, internal to the crate
// Not ported: ToTask_Scheduler_Resumed_On_Thread_Success_With_State_And_Cancellation - ToTask with a state object (AsyncState): the tasks of the port have no state
// Not ported: ToTask_Scheduler_Resumed_On_Thread_Failure_With_State_And_Cancellation - ToTask with a state object (AsyncState): the tasks of the port have no state
// Not ported: ToTask_Scheduler_Resumed_On_Thread_Cancel_With_State_And_Cancellation - ToTask with a state object (AsyncState): the tasks of the port have no state
// Not ported: ToTask_Scheduler_Dispose_Can_Propagate - real threads (Interval and ToTask on Scheduler.Default, Task.Delay, Thread.Sleep)
