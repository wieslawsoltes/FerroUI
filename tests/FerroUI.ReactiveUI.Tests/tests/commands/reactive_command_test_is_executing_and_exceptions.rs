//! `ReactiveCommandTest` (`ReactiveUI.Tests/Commands/ReactiveCommandTest.IsExecutingAndExceptions.cs`).
//!
//! The asynchronous tests of upstream wait for task completion sources with time-outs and delays
//! (`WaitAsync`, `Task.Delay(100)`); the tasks here run on the test's thread and continue as soon as
//! they are woken, so the tests check the state right after each step, and drive the worker-thread
//! delays with `block_on`.

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_reactiveui::*;
use ferroui_system::threading::tasks::{block_on, TaskCanceledError, TaskCompletionSource};
use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const TEST_ERROR_MESSAGE: &str = "Test error";
const TASK_EXCEPTION_MESSAGE: &str = "Task exception";
const STARTED_COMMAND_STATUS: &str = "started command";
const EXECUTING_FALSE_STATUS: &str = "executing = False";
const EXECUTING_TRUE_STATUS: &str = "executing = True";
const FINISHED_CANCELLING_STATUS: &str = "finished cancelling";
const CANCELLING_COMMAND_STATUS: &str = "cancelling command";

/// The trail of statuses of the task tests: each entry with its position.
type StatusTrail = Rc<RefCell<Vec<(usize, String)>>>;

/// Adds `status` at the next position (`statusTrail.Add((Interlocked.Increment(ref position) - 1, status))`).
fn push(trail: &StatusTrail, status: &str) {
    let mut trail = trail.borrow_mut();
    let position = trail.len();
    trail.push((position, status.to_owned()));
}

/// The status of a value of `IsExecuting` (`$"executing = {isExec}"`, with the `bool` of .NET).
fn executing_status(is_executing: bool) -> &'static str {
    if is_executing {
        EXECUTING_TRUE_STATUS
    } else {
        EXECUTING_FALSE_STATUS
    }
}

fn trail_of(entries: &[&str]) -> Vec<(usize, String)> {
    entries.iter().enumerate().map(|(position, status)| (position, (*status).to_owned())).collect()
}

#[test]
fn is_executing_handles_multiple_in_flight_executions() {
    let scheduler = TestScheduler::new();
    with_schedulers(scheduler.scheduler());
    let execute = Observable::return_(()).delay_on(Duration::from_millis(500), scheduler.scheduler());
    let command = ReactiveCommand::create_from_observable_with(move || execute.clone(), None, Some(scheduler.scheduler()));
    let executed = collect_items(&command.as_observable());

    let _ = command.execute_default().subscribe_ignore();
    let _ = command.execute_default().subscribe_ignore();
    scheduler.advance_by(duration_to_ticks(Duration::from_millis(1)));

    assert!(first(&command.is_executing()));
    assert!(executed.borrow().is_empty());
}

#[test]
fn is_executing_is_behavioral() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_with(|| {}, None, Some(immediate()));
    let is_executing = collect_items(&command.is_executing());

    assert_eq!(*is_executing.borrow(), [false]);
}

#[test]
fn is_executing_remains_true_until_execution_completes() {
    with_immediate_schedulers();
    let execute_subject = Subject::<()>::new();
    let source = execute_subject.as_observable();
    let command = ReactiveCommand::create_from_observable_with(move || source.clone(), None, Some(immediate()));

    let _ = command.execute_default().subscribe_ignore();

    assert!(first(&command.is_executing()));

    execute_subject.on_next(());
    assert!(first(&command.is_executing()));

    execute_subject.on_completed();
    assert!(!first(&command.is_executing()));
}

#[test]
fn is_executing_ticks_while_executing() {
    let scheduler = TestScheduler::new();
    with_schedulers(scheduler.scheduler());
    let execute = Observable::return_(()).delay_on(Duration::from_secs(1), scheduler.scheduler());
    let command = ReactiveCommand::create_from_observable_with(move || execute.clone(), None, Some(scheduler.scheduler()));
    let is_executing = collect_items(&command.is_executing());

    let _ = command.execute_default().subscribe_ignore();
    scheduler.advance_by(duration_to_ticks(Duration::from_millis(1)));

    assert_eq!(*is_executing.borrow(), [false, true]);
}

#[test]
fn observable_subscription_proper_lifecycle() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(0));
    let count = executed.clone();
    let command = ReactiveCommand::create_with(
        move || {
            count.set(count.get() + 1);
            count.get()
        },
        None,
        Some(immediate()),
    );

    let subscription = command.as_observable().subscribe_next(|_| {});
    first(&command.execute_default());

    assert_eq!(executed.get(), 1);

    subscription.dispose();
    first(&command.execute_default());

    // Should still execute even after subscription disposal.
    assert_eq!(executed.get(), 2);
}

#[test]
fn reactive_setpoint_async_method_execution() {
    let scheduler = TestScheduler::new();
    with_schedulers(scheduler.scheduler());

    let foo_vm = FooViewModel::new(Foo::new());

    assert_eq!(foo_vm.foo().value(), 42);

    scheduler.advance_by(duration_to_ticks(Duration::from_millis(11)));
    assert_eq!(foo_vm.foo().value(), 0);

    foo_vm.set_setpoint(123);
    scheduler.advance_by(duration_to_ticks(Duration::from_millis(5)));
    assert_eq!(foo_vm.foo().value(), 0);

    scheduler.advance_by(duration_to_ticks(Duration::from_millis(6)));
    assert_eq!(foo_vm.foo().value(), 123);
}

#[test]
fn scheduler_background_command_uses_background_scheduler() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_run_in_background_with(move || flag.set(true), None, Some(immediate()), Some(immediate()));

    first(&command.execute_default());

    assert!(executed.get());
}

#[test]
fn scheduler_results_delivered_on_output_scheduler() {
    let scheduler = with_immediate_schedulers();
    let command = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(scheduler));
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();

    let _ = command.execute_default().subscribe_next(move |_| flag.set(true));

    assert!(executed.get());
}

#[test]
fn task_cancellation_handles_proper_cancellation_flow() {
    with_immediate_schedulers();
    let tcs_started = TaskCompletionSource::<()>::new();
    let tcs_caught = TaskCompletionSource::<()>::new();
    let tcs_finish = TaskCompletionSource::<()>::new();
    let status_trail: StatusTrail = Rc::new(RefCell::new(Vec::new()));

    let (trail, started, caught, finish) = (status_trail.clone(), tcs_started.clone(), tcs_caught.clone(), tcs_finish.clone());
    let command = ReactiveCommand::create_from_task_cancellable_with(
        move |token| {
            let (trail, started, caught, finish) = (trail.clone(), started.clone(), caught.clone(), finish.clone());
            async move {
                push(&trail, STARTED_COMMAND_STATUS);
                started.try_set_result(());
                if let Err(error) = delay(Duration::from_millis(10_000), token).await {
                    push(&trail, CANCELLING_COMMAND_STATUS);
                    caught.try_set_result(());
                    finish.task().await?;
                    push(&trail, FINISHED_CANCELLING_STATUS);
                    return Err(error);
                }
                Ok(())
            }
        },
        None,
        Some(immediate()),
    );

    let exception: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let thrown = exception.clone();
    let _ = command.thrown_exceptions().subscribe_next(move |error| *thrown.borrow_mut() = Some(error));
    let latest_is_executing_value = Rc::new(Cell::new(false));
    let (trail, latest) = (status_trail.clone(), latest_is_executing_value.clone());
    let _ = command.is_executing().subscribe_next(move |is_executing| {
        push(&trail, executing_status(is_executing));
        latest.set(is_executing);
    });

    let disposable = subscribe_ignoring_errors(&command.execute_default());

    assert!(tcs_started.task().is_completed());
    assert!(latest_is_executing_value.get());

    disposable.dispose();

    assert!(tcs_caught.task().is_completed());
    assert!(latest_is_executing_value.get());

    tcs_finish.try_set_result(());

    assert!(!latest_is_executing_value.get());
    assert!(exception.borrow().as_ref().is_some_and(TaskCanceledError::is_cancellation));
    assert_eq!(
        *status_trail.borrow(),
        trail_of(&[
            EXECUTING_FALSE_STATUS,
            EXECUTING_TRUE_STATUS,
            STARTED_COMMAND_STATUS,
            CANCELLING_COMMAND_STATUS,
            FINISHED_CANCELLING_STATUS,
            EXECUTING_FALSE_STATUS,
        ])
    );
}

#[test]
fn task_completion_handles_proper_completion_flow() {
    with_immediate_schedulers();
    let tcs_started = TaskCompletionSource::<()>::new();
    let tcs_finished = TaskCompletionSource::<()>::new();
    let tcs_continue = TaskCompletionSource::<()>::new();
    let status_trail: StatusTrail = Rc::new(RefCell::new(Vec::new()));

    let (trail, started, finished, continue_) = (status_trail.clone(), tcs_started.clone(), tcs_finished.clone(), tcs_continue.clone());
    let command = ReactiveCommand::create_from_task_cancellable_with(
        move |token| {
            let (trail, started, finished, continue_) = (trail.clone(), started.clone(), finished.clone(), continue_.clone());
            async move {
                push(&trail, STARTED_COMMAND_STATUS);
                started.try_set_result(());
                if let Err(error) = delay(Duration::from_millis(1_000), token).await {
                    push(&trail, CANCELLING_COMMAND_STATUS);
                    delay(Duration::from_millis(5_000), Default::default()).await?;
                    push(&trail, FINISHED_CANCELLING_STATUS);
                    return Err(error);
                }
                push(&trail, "finished command");
                finished.try_set_result(());
                continue_.task().await?;
                Ok(())
            }
        },
        None,
        Some(immediate()),
    );

    let exception: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let thrown = exception.clone();
    let _ = command.thrown_exceptions().subscribe_next(move |error| *thrown.borrow_mut() = Some(error));
    let latest_is_executing_value = Rc::new(Cell::new(false));
    let (trail, latest) = (status_trail.clone(), latest_is_executing_value.clone());
    let _ = command.is_executing().subscribe_next(move |is_executing| {
        push(&trail, executing_status(is_executing));
        latest.set(is_executing);
    });

    let result = Rc::new(Cell::new(false));
    let flag = result.clone();
    let _ = command.execute_default().subscribe_next(move |_| flag.set(true));

    assert!(tcs_started.task().is_completed());
    assert!(latest_is_executing_value.get());

    block_on(tcs_finished.task()).expect("the command finishes");
    assert!(latest_is_executing_value.get());

    tcs_continue.try_set_result(());

    assert!(!latest_is_executing_value.get());
    assert!(result.get());
    assert!(exception.borrow().is_none());
    assert_eq!(
        *status_trail.borrow(),
        trail_of(&[EXECUTING_FALSE_STATUS, EXECUTING_TRUE_STATUS, STARTED_COMMAND_STATUS, "finished command", EXECUTING_FALSE_STATUS])
    );
}

#[test]
fn task_exception_handles_exception_flow() {
    with_immediate_schedulers();
    let tcs_start = TaskCompletionSource::<()>::new();
    let start = tcs_start.clone();
    let command = ReactiveCommand::create_from_task_cancellable_with(
        move |_token| {
            let start = start.clone();
            async move {
                start.task().await?;
                Err::<(), _>(InvalidOperationException::error(TASK_EXCEPTION_MESSAGE))
            }
        },
        None,
        Some(immediate()),
    );
    let is_executing = collect_items(&command.is_executing());
    let exceptions = collect_items(&command.thrown_exceptions());

    // Wait for the exception itself rather than a fixed delay.
    let thrown = command.thrown_exceptions().first_async().to_task();

    let _ = subscribe_ignoring_errors(&command.execute_default());

    tcs_start.set_result(());
    let _ = block_on(thrown).expect("the command throws");

    assert!(!is_executing.borrow()[0]);
    assert!(is_executing.borrow()[1]);
    assert_eq!(exceptions.borrow().len(), 1);
    assert_eq!(exceptions.borrow()[0].to_string(), TASK_EXCEPTION_MESSAGE);
}

#[test]
fn thrown_exceptions_captures_lambda_exceptions() {
    with_immediate_schedulers();
    let command =
        ReactiveCommand::try_create_from_observable_with::<()>(|| Err(InvalidOperationException::error("Lambda error")), None, Some(immediate()));
    let exceptions = collect_items(&command.thrown_exceptions());

    let _ = subscribe_ignoring_errors(&command.execute_default());

    assert_eq!(exceptions.borrow().len(), 1);
    assert!(is_invalid_operation(&exceptions.borrow()[0]));
    assert_eq!(exceptions.borrow()[0].to_string(), "Lambda error");
}

#[test]
fn thrown_exceptions_captures_observable_exceptions() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_from_observable_with(
        || Observable::throw::<()>(InvalidOperationException::error(TEST_ERROR_MESSAGE)),
        None,
        Some(immediate()),
    );
    let exceptions = collect_items(&command.thrown_exceptions());

    let _ = subscribe_ignoring_errors(&command.execute_default());

    assert_eq!(exceptions.borrow().len(), 1);
    assert!(is_invalid_operation(&exceptions.borrow()[0]));
    assert_eq!(exceptions.borrow()[0].to_string(), TEST_ERROR_MESSAGE);
}

#[test]
fn thrown_exceptions_delivered_on_output_scheduler() {
    let scheduler = with_immediate_schedulers();
    let command = ReactiveCommand::create_from_observable_with(|| Observable::throw::<()>(InvalidOperationException::error("")), None, Some(scheduler));
    let exception: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let thrown = exception.clone();
    let _ = command.thrown_exceptions().subscribe_next(move |error| *thrown.borrow_mut() = Some(error));

    let _ = subscribe_ignoring_errors(&command.execute_default());

    assert!(exception.borrow().as_ref().is_some_and(is_invalid_operation));
}

#[test]
fn thrown_exceptions_propagates_task_exceptions() {
    with_immediate_schedulers();
    let tcs_start = TaskCompletionSource::<()>::new();
    let start = tcs_start.clone();
    let command = ReactiveCommand::create_from_task_cancellable_with(
        move |_token| {
            let start = start.clone();
            async move {
                start.task().await?;
                Err::<(), _>(InvalidOperationException::error(TASK_EXCEPTION_MESSAGE))
            }
        },
        None,
        Some(immediate()),
    );
    let exceptions = collect_items(&command.thrown_exceptions());

    let _ = subscribe_ignoring_errors(&command.execute_default());

    tcs_start.set_result(());

    assert_eq!(exceptions.borrow().len(), 1);
    assert_eq!(exceptions.borrow()[0].to_string(), TASK_EXCEPTION_MESSAGE);
}
