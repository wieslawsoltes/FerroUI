//! `ReactiveCommandTest` (`ReactiveUI.Tests/Commands/ReactiveCommandTest.CanExecuteOverloads.cs`):
//! the factories with a gate and no output scheduler, whose streams use
//! `RxSchedulers::main_thread_scheduler` (the immediate scheduler, as `WithSchedulerExecutor` sets
//! it).

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactiveui::*;
use ferroui_system::threading::tasks::Task;
use ferroui_system::IObserver;
use std::cell::Cell;
use std::rc::Rc;

const PARAMETER_VALUE: i32 = 42;
const PARAMETER_VALUE_STRING: &str = "42";

/// `AssertGatedThenOpenAsync`: the command cannot execute until the gate opens.
fn assert_gated_then_open(command: &dyn ICommand, can_execute: &Rc<BehaviorSubject<bool>>) {
    assert!(!command.can_execute(None));

    can_execute.on_next(true);

    assert!(command.can_execute(None));
}

#[test]
fn create_action_with_param_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let received = Rc::new(Cell::new(0));
    let param = received.clone();
    let command = ReactiveCommand::create_param_with(move |parameter: i32| param.set(parameter), Some(can_execute.as_observable()), None);

    assert_gated_then_open(&command, &can_execute);
    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received.get(), PARAMETER_VALUE);
}

#[test]
fn create_func_with_param_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_param_with(|parameter: i32| parameter.to_string(), Some(can_execute.as_observable()), None);

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute(PARAMETER_VALUE));

    assert_eq!(result, PARAMETER_VALUE_STRING);
}

#[test]
fn create_run_in_background_action_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_run_in_background_with(move || flag.set(true), Some(can_execute.as_observable()), None, None);

    assert_gated_then_open(&command, &can_execute);
    first(&command.execute_default());

    assert!(executed.get());
}

#[test]
fn create_run_in_background_action_can_execute_and_background_scheduler_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_run_in_background_with(move || flag.set(true), Some(can_execute.as_observable()), Some(immediate()), None);

    assert_gated_then_open(&command, &can_execute);
    first(&command.execute_default());

    assert!(executed.get());
}

#[test]
fn create_run_in_background_action_background_and_output_schedulers_executes() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_run_in_background_with(move || flag.set(true), None, Some(immediate()), Some(immediate()));

    first(&command.execute_default());

    assert!(executed.get());
}

#[test]
fn create_run_in_background_func_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_run_in_background_with(|| PARAMETER_VALUE, Some(can_execute.as_observable()), None, None);

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute_default());

    assert_eq!(result, PARAMETER_VALUE);
}

#[test]
fn create_run_in_background_func_can_execute_and_background_scheduler_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_run_in_background_with(|| PARAMETER_VALUE, Some(can_execute.as_observable()), Some(immediate()), None);

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute_default());

    assert_eq!(result, PARAMETER_VALUE);
}

#[test]
fn create_run_in_background_action_with_param_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let received = Rc::new(Cell::new(0));
    let param = received.clone();
    let command = ReactiveCommand::create_run_in_background_param_with(move |parameter: i32| param.set(parameter), Some(can_execute.as_observable()), None, None);

    assert_gated_then_open(&command, &can_execute);
    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received.get(), PARAMETER_VALUE);
}

#[test]
fn create_run_in_background_action_with_param_can_execute_and_background_scheduler_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let received = Rc::new(Cell::new(0));
    let param = received.clone();
    let command = ReactiveCommand::create_run_in_background_param_with(
        move |parameter: i32| param.set(parameter),
        Some(can_execute.as_observable()),
        Some(immediate()),
        None,
    );

    assert_gated_then_open(&command, &can_execute);
    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received.get(), PARAMETER_VALUE);
}

#[test]
fn create_run_in_background_func_with_param_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_run_in_background_param_with(|parameter: i32| parameter.to_string(), Some(can_execute.as_observable()), None, None);

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute(PARAMETER_VALUE));

    assert_eq!(result, PARAMETER_VALUE_STRING);
}

#[test]
fn create_run_in_background_func_with_param_can_execute_and_background_scheduler_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_run_in_background_param_with(
        |parameter: i32| parameter.to_string(),
        Some(can_execute.as_observable()),
        Some(immediate()),
        None,
    );

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute(PARAMETER_VALUE));

    assert_eq!(result, PARAMETER_VALUE_STRING);
}

#[test]
fn create_combined_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let child1 = ReactiveCommand::create_param_with(|parameter: i32| parameter, None, Some(immediate()));
    let child2 = ReactiveCommand::create_param_with(|parameter: i32| parameter + 1, None, Some(immediate()));
    let command = ReactiveCommand::create_combined_with([child1, child2], Some(can_execute.as_observable()), None);

    assert_gated_then_open(&command, &can_execute);
    let results = first(&command.execute(PARAMETER_VALUE));

    assert_eq!(results, [PARAMETER_VALUE, PARAMETER_VALUE + 1]);
}

#[test]
fn create_from_observable_with_param_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command =
        ReactiveCommand::create_from_observable_param_with(|parameter: i32| Observable::return_(parameter.to_string()), Some(can_execute.as_observable()), None);

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute(PARAMETER_VALUE));

    assert_eq!(result, PARAMETER_VALUE_STRING);
}

#[test]
fn create_from_task_result_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_from_task_with(|| Task::from_result(PARAMETER_VALUE), Some(can_execute.as_observable()), None);

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute_default());

    assert_eq!(result, PARAMETER_VALUE);
}

#[test]
fn create_from_task_cancellable_result_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_from_task_cancellable_with(|_| Task::from_result(PARAMETER_VALUE), Some(can_execute.as_observable()), None);

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute_default());

    assert_eq!(result, PARAMETER_VALUE);
}

#[test]
fn create_from_task_unit_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_from_task_with(
        move || {
            flag.set(true);
            Task::from_result(())
        },
        Some(can_execute.as_observable()),
        None,
    );

    assert_gated_then_open(&command, &can_execute);
    first(&command.execute_default());

    assert!(executed.get());
}

#[test]
fn create_from_task_cancellable_unit_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_from_task_cancellable_with(
        move |_| {
            flag.set(true);
            Task::from_result(())
        },
        Some(can_execute.as_observable()),
        None,
    );

    assert_gated_then_open(&command, &can_execute);
    first(&command.execute_default());

    assert!(executed.get());
}

#[test]
fn create_from_task_param_result_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command =
        ReactiveCommand::create_from_task_param_with(|parameter: i32| Task::from_result(parameter.to_string()), Some(can_execute.as_observable()), None);

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute(PARAMETER_VALUE));

    assert_eq!(result, PARAMETER_VALUE_STRING);
}

#[test]
fn create_from_task_cancellable_param_result_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_from_task_param_cancellable_with(
        |parameter: i32, _| Task::from_result(parameter.to_string()),
        Some(can_execute.as_observable()),
        None,
    );

    assert_gated_then_open(&command, &can_execute);
    let result = first(&command.execute(PARAMETER_VALUE));

    assert_eq!(result, PARAMETER_VALUE_STRING);
}

#[test]
fn create_from_task_param_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let received = Rc::new(Cell::new(0));
    let param = received.clone();
    let command = ReactiveCommand::create_from_task_param_with(
        move |parameter: i32| {
            param.set(parameter);
            Task::from_result(())
        },
        Some(can_execute.as_observable()),
        None,
    );

    assert_gated_then_open(&command, &can_execute);
    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received.get(), PARAMETER_VALUE);
}

#[test]
fn create_from_task_cancellable_param_can_execute_gates_and_executes() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let received = Rc::new(Cell::new(0));
    let param = received.clone();
    let command = ReactiveCommand::create_from_task_param_cancellable_with(
        move |parameter: i32, _| {
            param.set(parameter);
            Task::from_result(())
        },
        Some(can_execute.as_observable()),
        None,
    );

    assert_gated_then_open(&command, &can_execute);
    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received.get(), PARAMETER_VALUE);
}
