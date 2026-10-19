//! Ported from `Tests.System.Reactive/Tests/TaskObservableExtensionsTest.cs` of Rx.NET.
//! Adapted: upstream runs the body of the `TaskToObservable_*` tests through a `TaskFactory` of a
//! `TestTaskScheduler`, which runs it inline; here the body runs directly, through
//! `observe_task`. The `Task` forms without a result (`((Task)taskSource.Task).ToObservable()`)
//! are the same methods with `T = ()`, on a `TaskCompletionSource<()>`. `InvalidOperationException`
//! of a sequence without values is `SequenceError::NoElements`, and the errors of the tests are
//! checked by identity. The `DESKTOPCLR` tests are ported; their wait for the event is not needed,
//! since the continuation runs when the source is completed. The cancellation error of the port
//! carries no token, so `new TaskCanceledException(task).CancellationToken` is not compared.

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction, Scheduler};
use ferroui_reactive::linq::SequenceError;
use ferroui_reactive::{IDisposable, Observable, ObservableError, ObservableExt, TaskObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, MockObserver, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::tasks::{TaskCanceledError, TaskCompletionSource};
use ferroui_system::threading::CancellationTokenSource;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::thread::{self, ThreadId};
use std::time::{Duration, SystemTime};

/// `Assert.Same` of two errors.
fn same(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// The body of the `TaskToObservable_*` tests: a completion source whose task is observed
/// (`ContinueWith(t => { var e = t.Exception; })`), ended by `end` at `time`, and the observer of
/// `taskSource.Task.ToObservable()` created at 100, subscribed at 200 and disposed at 1000.
fn observe_task<T: Clone + 'static>(time: i64, end: impl FnOnce(&TaskCompletionSource<T>) + 'static) -> Rc<MockObserver<T>> {
    let scheduler = TestScheduler::new();

    let task_source = TaskCompletionSource::<T>::new();
    task_source.task().continue_with(|t| {
        let _e = t.exception();
    });

    let source = task_source.clone();
    scheduler.schedule_absolute_action(time, move || end(&source));

    scheduler.start(move || task_source.task().to_observable())
}

#[test]
fn task_to_observable_non_void_complete_before_create() {
    let res = observe_task::<i32>(10, |task_source| task_source.set_result(42));

    assert_messages!(res.messages(), RT::on_next(200, 42), RT::on_completed::<i32>(200));
}

#[test]
fn task_to_observable_non_void_complete_before_subscribe() {
    let res = observe_task::<i32>(110, |task_source| task_source.set_result(42));

    assert_messages!(res.messages(), RT::on_next(200, 42), RT::on_completed::<i32>(200));
}

#[test]
fn task_to_observable_non_void_complete_before_dispose() {
    let res = observe_task::<i32>(300, |task_source| task_source.set_result(42));

    assert_messages!(res.messages(), RT::on_next(300, 42), RT::on_completed::<i32>(300));
}

#[test]
fn task_to_observable_non_void_complete_after_dispose() {
    let res = observe_task::<i32>(1100, |task_source| task_source.set_result(42));

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_non_void_exception_before_create() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<i32>(10, move |task_source| task_source.set_exception(error));

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));
}

#[test]
fn task_to_observable_non_void_exception_before_subscribe() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<i32>(110, move |task_source| task_source.set_exception(error));

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));
}

#[test]
fn task_to_observable_non_void_exception_before_dispose() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<i32>(300, move |task_source| task_source.set_exception(error));

    assert_messages!(res.messages(), RT::on_error::<i32>(300, ex));
}

#[test]
fn task_to_observable_non_void_exception_after_dispose() {
    let res = observe_task::<i32>(1100, |task_source| task_source.set_exception(new_error("ex")));

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_non_void_canceled_before_create() {
    let res = observe_task::<i32>(10, |task_source| task_source.set_canceled());

    assert_messages!(res.messages(), RT::on_error_where::<i32>(200, TaskCanceledError::is_cancellation));
}

#[test]
fn task_to_observable_non_void_canceled_before_subscribe() {
    let res = observe_task::<i32>(110, |task_source| task_source.set_canceled());

    assert_messages!(res.messages(), RT::on_error_where::<i32>(200, TaskCanceledError::is_cancellation));
}

#[test]
fn task_to_observable_non_void_canceled_before_dispose() {
    let res = observe_task::<i32>(300, |task_source| task_source.set_canceled());

    assert_messages!(res.messages(), RT::on_error_where::<i32>(300, TaskCanceledError::is_cancellation));
}

#[test]
fn task_to_observable_non_void_canceled_after_dispose() {
    let res = observe_task::<i32>(1100, |task_source| task_source.set_canceled());

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_non_void_scheduler() {
    let x = Rc::new(Cell::new(0));
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let cts = TaskCompletionSource::<i32>::new();

    let xs = cts.task().to_observable_on(Scheduler::immediate());
    let _d = xs.subscribe_next({
        let (x, t) = (x.clone(), t.clone());
        move |res| {
            x.set(res);
            t.set(Some(thread::current().id()));
        }
    });

    cts.set_result(42);

    assert_eq!(42, x.get());
    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn task_to_observable_void_complete_before_create() {
    let res = observe_task::<()>(10, |task_source| task_source.set_result(()));

    assert_messages!(res.messages(), RT::on_next(200, ()), RT::on_completed::<()>(200));
}

#[test]
fn task_to_observable_void_complete_before_subscribe() {
    let res = observe_task::<()>(110, |task_source| task_source.set_result(()));

    assert_messages!(res.messages(), RT::on_next(200, ()), RT::on_completed::<()>(200));
}

#[test]
fn task_to_observable_void_complete_before_dispose() {
    let res = observe_task::<()>(300, |task_source| task_source.set_result(()));

    assert_messages!(res.messages(), RT::on_next(300, ()), RT::on_completed::<()>(300));
}

#[test]
fn task_to_observable_void_complete_after_dispose() {
    let res = observe_task::<()>(1100, |task_source| task_source.set_result(()));

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_void_exception_before_create() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<()>(10, move |task_source| task_source.set_exception(error));

    assert_messages!(res.messages(), RT::on_error::<()>(200, ex));
}

#[test]
fn task_to_observable_void_exception_before_subscribe() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<()>(110, move |task_source| task_source.set_exception(error));

    assert_messages!(res.messages(), RT::on_error::<()>(200, ex));
}

#[test]
fn task_to_observable_void_exception_before_dispose() {
    let ex = new_error("ex");

    let error = ex.clone();
    let res = observe_task::<()>(300, move |task_source| task_source.set_exception(error));

    assert_messages!(res.messages(), RT::on_error::<()>(300, ex));
}

#[test]
fn task_to_observable_void_exception_after_dispose() {
    let res = observe_task::<()>(1100, |task_source| task_source.set_exception(new_error("ex")));

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_void_canceled_before_create() {
    let res = observe_task::<()>(10, |task_source| task_source.set_canceled());

    assert_messages!(res.messages(), RT::on_error_where::<()>(200, TaskCanceledError::is_cancellation));
}

#[test]
fn task_to_observable_void_canceled_before_subscribe() {
    let res = observe_task::<()>(110, |task_source| task_source.set_canceled());

    assert_messages!(res.messages(), RT::on_error_where::<()>(200, TaskCanceledError::is_cancellation));
}

#[test]
fn task_to_observable_void_canceled_before_dispose() {
    let res = observe_task::<()>(300, |task_source| task_source.set_canceled());

    assert_messages!(res.messages(), RT::on_error_where::<()>(300, TaskCanceledError::is_cancellation));
}

#[test]
fn task_to_observable_void_canceled_after_dispose() {
    let res = observe_task::<()>(1100, |task_source| task_source.set_canceled());

    assert_messages!(res.messages());
}

#[test]
fn task_to_observable_void_scheduler() {
    let t: Rc<Cell<Option<ThreadId>>> = Rc::new(Cell::new(None));

    let tcs = TaskCompletionSource::<()>::new();

    let xs = tcs.task().to_observable_on(Scheduler::immediate());
    let _d = xs.subscribe_next({
        let t = t.clone();
        move |_| t.set(Some(thread::current().id()))
    });

    tcs.set_result(());

    assert_eq!(Some(thread::current().id()), t.get());
}

#[test]
fn observable_to_task_no_value() {
    let scheduler = TestScheduler::new();

    let xs = Observable::empty_on::<i32>(scheduler.scheduler());

    let continuation = xs.to_task();
    scheduler.run();

    assert!(continuation.is_faulted());
    let error = continuation.exception().expect("the task is faulted");
    assert_eq!(Some(&SequenceError::NoElements), error.downcast_ref::<SequenceError>());
    assert_eq!(1, scheduler.clock());
}

#[test]
fn observable_to_task_single_value() {
    let scheduler = TestScheduler::new();

    let xs = Observable::return_on(5, scheduler.scheduler());

    let continuation = xs.to_task();
    scheduler.run();

    assert!(continuation.is_completed());
    assert_eq!(5, continuation.result().unwrap());
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

    assert!(continuation.is_completed());
    assert_eq!(3, continuation.result().unwrap());

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

    assert!(continuation.is_faulted());
    let ag = continuation.exception();
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

    let cts = CancellationTokenSource::new();

    let continuation = xs.as_observable().to_task_with_token(cts.token());
    {
        let cts = cts.clone();
        scheduler.schedule_absolute_action(125, move || cts.cancel());
    }
    scheduler.run();

    assert!(continuation.is_canceled());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(0, 125));
}

#[test]
fn to_task_scheduler_resumed_on_thread_success() {
    let scheduler = OneshotScheduler::new();

    let task = Observable::return_(1).to_task_on(scheduler.clone());

    assert!(scheduler.has_task());

    scheduler.run();

    assert!(!scheduler.has_task());

    assert!(task.is_completed());
}

#[test]
fn to_task_scheduler_resumed_on_thread_failure() {
    let scheduler = OneshotScheduler::new();

    let ex = new_error("failure");
    let task = Observable::throw::<i32>(ex.clone()).to_task_on(scheduler.clone());

    assert!(scheduler.has_task());

    scheduler.run();

    assert!(!scheduler.has_task());

    assert!(task.is_faulted());
    let error = task.exception().expect("the task is faulted");
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

    assert!(task.is_completed());
}

#[test]
fn to_task_scheduler_resumed_on_thread_failure_with_cancellation() {
    let scheduler = OneshotScheduler::new();

    let ex = new_error("failure");
    let task = Observable::throw::<i32>(ex.clone()).to_task_with_token_on(Default::default(), scheduler.clone());

    assert!(scheduler.has_task());

    scheduler.run();

    assert!(!scheduler.has_task());

    assert!(task.is_faulted());
    let error = task.exception().expect("the task is faulted");
    assert!(same(&ex, &error));
    assert_eq!("failure", error.to_string());
}

#[test]
fn to_task_scheduler_resumed_on_thread_cancel_with_cancellation() {
    let scheduler = OneshotScheduler::new();

    let cts = CancellationTokenSource::new();

    let task = Observable::never::<i32>().to_task_with_token_on(cts.token(), scheduler.clone());

    assert!(!scheduler.has_task());

    cts.cancel();

    assert!(scheduler.has_task());

    scheduler.run();

    assert!(!scheduler.has_task());

    assert!(task.is_canceled());
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
