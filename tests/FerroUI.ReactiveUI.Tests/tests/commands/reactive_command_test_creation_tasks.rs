//! `ReactiveCommandTest` (`ReactiveUI.Tests/Commands/ReactiveCommandTest.CreationTasks.cs`).
//!
//! `Task.Delay(ms, token)` is [`delay`], whose time passes on a worker thread; [`first`] runs the
//! tasks of the thread until the execution notifies. An `async` lambda is an `async` block, and
//! `Task.FromResult` is `Task::from_result`.

use crate::mocks::*;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const PARAMETER_VALUE: i32 = 42;
const PARAMETER_VALUE_STRING: &str = "42";
const DELAY: Duration = Duration::from_millis(10);

#[test]
fn create_from_task_cancellable_properly_cancels_execution() {
    with_immediate_schedulers();
    let tcs_started = TaskCompletionSource::<()>::new();
    let tcs_caught = TaskCompletionSource::<()>::new();
    let tcs_finish = TaskCompletionSource::<()>::new();

    let (started, caught, finish) = (tcs_started.clone(), tcs_caught.clone(), tcs_finish.clone());
    let fixture = ReactiveCommand::create_from_task_cancellable_with(
        move |token| {
            let (started, caught, finish) = (started.clone(), caught.clone(), finish.clone());
            async move {
                started.try_set_result(());
                if let Err(error) = delay(Duration::from_millis(10_000), token).await {
                    caught.try_set_result(());
                    finish.task().await?;
                    return Err(error);
                }
                Ok(())
            }
        },
        None,
        Some(immediate()),
    );

    let _ = fixture.thrown_exceptions().subscribe_next(|_| {});

    let disposable = subscribe_ignoring_errors(&fixture.execute_default());

    assert!(tcs_started.task().is_completed());
    disposable.dispose();

    assert!(tcs_caught.task().is_completed());
    tcs_finish.try_set_result(());

    // The cancellation has completed: the command is no longer executing.
    assert!(!first(&fixture.is_executing()));
}

#[test]
fn create_from_task_cancellable_unit_without_param_receives_cancellation_token() {
    with_immediate_schedulers();
    let received_token: Rc<RefCell<Option<CancellationToken>>> = Rc::new(RefCell::new(None));
    let received = received_token.clone();
    let command = ReactiveCommand::create_from_task_cancellable_with(
        move |token: CancellationToken| {
            *received.borrow_mut() = Some(token.clone());
            delay(DELAY, token)
        },
        None,
        Some(immediate()),
    );

    first(&command.execute_default());

    assert!(received_token.borrow().is_some());
}

// Not ported: CreateFromTask_Cancellable_Unit_WithoutParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_from_task_cancellable_unit_with_param_receives_parameter_and_token() {
    with_immediate_schedulers();
    let received_param = Rc::new(Cell::new(0));
    let received_token: Rc<RefCell<Option<CancellationToken>>> = Rc::new(RefCell::new(None));
    let (param, received) = (received_param.clone(), received_token.clone());
    let command = ReactiveCommand::create_from_task_param_cancellable_with(
        move |parameter: i32, token: CancellationToken| {
            param.set(parameter);
            *received.borrow_mut() = Some(token.clone());
            delay(DELAY, token)
        },
        None,
        Some(immediate()),
    );

    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received_param.get(), PARAMETER_VALUE);
    assert!(received_token.borrow().is_some());
}

// Not ported: CreateFromTask_Cancellable_Unit_WithParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_from_task_cancellable_without_param_receives_cancellation_token() {
    with_immediate_schedulers();
    let received_token: Rc<RefCell<Option<CancellationToken>>> = Rc::new(RefCell::new(None));
    let received = received_token.clone();
    let command = ReactiveCommand::create_from_task_cancellable_with(
        move |token: CancellationToken| {
            *received.borrow_mut() = Some(token.clone());
            async move {
                delay(DELAY, token).await?;
                Ok(PARAMETER_VALUE)
            }
        },
        None,
        Some(immediate()),
    );

    first(&command.execute_default());

    assert!(received_token.borrow().is_some());
}

// Not ported: CreateFromTask_Cancellable_WithoutParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_from_task_cancellable_with_param_receives_parameter_and_token() {
    with_immediate_schedulers();
    let received_param = Rc::new(Cell::new(0));
    let received_token: Rc<RefCell<Option<CancellationToken>>> = Rc::new(RefCell::new(None));
    let (param, received) = (received_param.clone(), received_token.clone());
    let command = ReactiveCommand::create_from_task_param_cancellable_with(
        move |parameter: i32, token: CancellationToken| {
            param.set(parameter);
            *received.borrow_mut() = Some(token.clone());
            async move {
                delay(DELAY, token).await?;
                Ok(parameter.to_string())
            }
        },
        None,
        Some(immediate()),
    );

    // Capture the result straight from the execution stream: for an asynchronous command the
    // per-execution value comes before the shared results broadcast.
    let result = first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received_param.get(), PARAMETER_VALUE);
    assert!(received_token.borrow().is_some());
    assert_eq!(result, PARAMETER_VALUE_STRING);
}

// Not ported: CreateFromTask_Cancellable_WithParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_from_task_unit_without_param_completes_successfully() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_from_task_with(
        move || {
            let flag = flag.clone();
            async move {
                flag.set(true);
                Ok(())
            }
        },
        None,
        Some(immediate()),
    );

    first(&command.execute_default());

    assert!(executed.get());
}

// Not ported: CreateFromTask_Unit_WithoutParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_from_task_unit_with_param_passes_parameter() {
    with_immediate_schedulers();
    let received_param = Rc::new(Cell::new(0));
    let param = received_param.clone();
    let command = ReactiveCommand::create_from_task_param_with(
        move |parameter: i32| {
            let param = param.clone();
            async move {
                param.set(parameter);
                Ok(())
            }
        },
        None,
        Some(immediate()),
    );

    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received_param.get(), PARAMETER_VALUE);
}

// Not ported: CreateFromTask_Unit_WithParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_from_task_without_param_returns_task_result() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_from_task_with(|| Task::from_result(PARAMETER_VALUE), None, Some(immediate()));
    let results = collect_items(&command.as_observable());

    first(&command.execute_default());

    assert_eq!(results.borrow()[0], PARAMETER_VALUE);
}

// Not ported: CreateFromTask_WithoutParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_from_task_with_param_passes_parameter_to_task() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_from_task_param_with(|parameter: i32| Task::from_result(parameter.to_string()), None, Some(immediate()));
    let results = collect_items(&command.as_observable());

    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(results.borrow()[0], PARAMETER_VALUE_STRING);
}

// Not ported: CreateFromTask_WithParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_run_in_background_action_executes_on_background_scheduler() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_run_in_background_with(move || flag.set(true), None, Some(immediate()), Some(immediate()));

    first(&command.execute_default());

    assert!(executed.get());
}

// Not ported: CreateRunInBackground_Action_ThrowsOnNullExecute - argument validation

#[test]
fn create_run_in_background_action_with_param_passes_parameter() {
    with_immediate_schedulers();
    let received_param = Rc::new(Cell::new(0));
    let param = received_param.clone();
    let command = ReactiveCommand::create_run_in_background_param_with(move |parameter: i32| param.set(parameter), None, Some(immediate()), Some(immediate()));

    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received_param.get(), PARAMETER_VALUE);
}

// Not ported: CreateRunInBackground_ActionWithParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_run_in_background_func_returns_result() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_run_in_background_with(|| PARAMETER_VALUE, None, Some(immediate()), Some(immediate()));
    let results = collect_items(&command.as_observable());

    first(&command.execute_default());

    assert_eq!(results.borrow()[0], PARAMETER_VALUE);
}

// Not ported: CreateRunInBackground_Func_ThrowsOnNullExecute - argument validation
// Not ported: CreateRunInBackground_FuncWithParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_run_in_background_func_with_param_transforms_parameter() {
    with_immediate_schedulers();
    let command =
        ReactiveCommand::create_run_in_background_param_with(|parameter: i32| parameter.to_string(), None, Some(immediate()), Some(immediate()));
    let results = collect_items(&command.as_observable());

    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(results.borrow()[0], PARAMETER_VALUE_STRING);
}
