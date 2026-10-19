//! `ReactiveCommandTest` (`ReactiveUI.Tests/Commands/ReactiveCommandTest.cs`).
//!
//! Upstream runs the class under `WithSchedulerExecutor`, which makes the schedulers of
//! `RxSchedulers` the immediate scheduler; each test here starts with
//! [`with_immediate_schedulers`]. The tests marked `WithVirtualTimeSchedulerExecutor` use a
//! `TestScheduler` instead. A function that throws upstream is the `try_` form of its factory,
//! returning an `Err`. `await x.FirstAsync()` is [`first`].

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::{BehaviorSubject, Subject, SubjectBase};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const PARAMETER_VALUE: i32 = 42;
const PARAMETER_VALUE_STRING: &str = "42";
const TEST_ERROR_MESSAGE: &str = "Test error";

#[test]
fn can_execute_is_behavioral() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_with(|| {}, None, Some(immediate()));
    let can_execute = collect_items(&command.can_execute());

    assert_eq!(*can_execute.borrow(), [true]);
}

#[test]
fn can_execute_is_false_while_executing() {
    let scheduler = TestScheduler::new();
    with_schedulers(scheduler.scheduler());
    let execute = Observable::return_(()).delay_on(Duration::from_secs(1), scheduler.scheduler());
    let command = ReactiveCommand::create_from_observable_with(move || execute.clone(), None, Some(scheduler.scheduler()));
    let can_execute = collect_items(&command.can_execute());

    let _ = command.execute_default().subscribe_ignore();
    scheduler.advance_by(duration_to_ticks(Duration::from_millis(1)));

    assert_eq!(*can_execute.borrow(), [true, false]);
}

#[test]
fn can_execute_only_ticks_distinct_values() {
    with_immediate_schedulers();
    let can_execute_subject = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_with(|| {}, Some(can_execute_subject.as_observable()), Some(immediate()));
    let can_execute = collect_items(&command.can_execute());

    can_execute_subject.on_next(false);
    can_execute_subject.on_next(false);
    can_execute_subject.on_next(true);
    can_execute_subject.on_next(true);

    assert_eq!(*can_execute.borrow(), [false, true]);
}

#[test]
fn can_execute_respects_provided_observable() {
    with_immediate_schedulers();
    let can_execute_subject = BehaviorSubject::new(false);
    let command = ReactiveCommand::create_with(|| {}, Some(can_execute_subject.as_observable()), Some(immediate()));
    let can_execute = collect_items(&command.can_execute());

    can_execute_subject.on_next(true);
    can_execute_subject.on_next(false);

    assert_eq!(*can_execute.borrow(), [false, true, false]);
}

#[test]
fn can_execute_ticks_exceptions_through_thrown_exceptions() {
    with_immediate_schedulers();
    let can_execute_subject = Subject::<bool>::new();
    let command = ReactiveCommand::create_with(|| {}, Some(can_execute_subject.as_observable()), Some(immediate()));
    let exceptions = collect_items(&command.thrown_exceptions());

    can_execute_subject.on_error(InvalidOperationException::error(TEST_ERROR_MESSAGE));

    assert_eq!(exceptions.borrow().len(), 1);
    assert!(is_invalid_operation(&exceptions.borrow()[0]));
}

#[test]
fn can_execute_unsubscribes_on_disposal() {
    with_immediate_schedulers();
    let can_execute_subject = BehaviorSubject::new(true);
    let command = ReactiveCommand::create_with(|| {}, Some(can_execute_subject.as_observable()), Some(immediate()));

    assert!(can_execute_subject.has_observers());

    command.dispose();

    assert!(!can_execute_subject.has_observers());
}

#[test]
fn create_action_executes_successfully() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_with(move || flag.set(true), None, Some(immediate()));

    first(&command.execute_default());

    assert!(executed.get());
}

#[test]
fn create_action_respects_can_execute() {
    with_immediate_schedulers();
    let can_execute = BehaviorSubject::new(false);
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command = ReactiveCommand::create_with(move || flag.set(true), Some(can_execute.as_observable()), Some(immediate()));
    let source = Subject::<()>::new();

    let _ = source.as_observable().invoke_command(command);

    source.on_next(());
    assert!(!executed.get());

    can_execute.on_next(true);
    source.on_next(());
    assert!(executed.get());
}

// Not ported: Create_Action_ThrowsOnNullExecute - argument validation

#[test]
fn create_action_with_param_handles_multiple_executions() {
    with_immediate_schedulers();
    let parameters = Rc::new(RefCell::new(Vec::new()));
    let sink = parameters.clone();
    let command = ReactiveCommand::create_param_with(move |parameter: i32| sink.borrow_mut().push(parameter), None, Some(immediate()));

    first(&command.execute(1));
    first(&command.execute(2));
    first(&command.execute(3));

    assert_eq!(*parameters.borrow(), [1, 2, 3]);
}

#[test]
fn create_action_with_param_passes_parameter_correctly() {
    with_immediate_schedulers();
    let received_param = Rc::new(Cell::new(0));
    let received = received_param.clone();
    let command = ReactiveCommand::create_param_with(move |parameter: i32| received.set(parameter), None, Some(immediate()));

    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(received_param.get(), PARAMETER_VALUE);
}

// Not ported: Create_ActionWithParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_func_returns_result() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_with(|| PARAMETER_VALUE, None, Some(immediate()));
    let results = collect_items(&command.as_observable());

    first(&command.execute_default());

    assert_eq!(*results.borrow(), [PARAMETER_VALUE]);
}

// Not ported: Create_Func_ThrowsOnNullExecute - argument validation

#[test]
fn create_func_ticks_multiple_results() {
    with_immediate_schedulers();
    let counter = Rc::new(Cell::new(0));
    let count = counter.clone();
    let command = ReactiveCommand::create_with(
        move || {
            count.set(count.get() + 1);
            count.get()
        },
        None,
        Some(immediate()),
    );
    let results = collect_items(&command.as_observable());

    first(&command.execute_default());
    first(&command.execute_default());
    first(&command.execute_default());

    assert_eq!(*results.borrow(), [1, 2, 3]);
}

#[test]
fn create_func_with_param_returns_result_from_parameter() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_param_with(|parameter: i32| parameter.to_string(), None, Some(immediate()));
    let results = collect_items(&command.as_observable());

    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(results.borrow()[0], PARAMETER_VALUE_STRING);
}

// Not ported: Create_FuncWithParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_func_with_param_transforms_parameters() {
    with_immediate_schedulers();
    const MULTIPLIER: i32 = 2;
    let command = ReactiveCommand::create_param_with(|parameter: i32| parameter * MULTIPLIER, None, Some(immediate()));
    let results = collect_items(&command.as_observable());

    first(&command.execute(5));
    first(&command.execute(10));
    first(&command.execute(15));

    assert_eq!(*results.borrow(), [10, 20, 30]);
}

#[test]
fn create_combined_can_execute_is_false_if_any_child_cannot_execute() {
    with_immediate_schedulers();
    let can_execute1 = BehaviorSubject::new(true);
    let can_execute2 = BehaviorSubject::new(false);
    let cmd1 = ReactiveCommand::create_param_with(|x: i32| x, Some(can_execute1.as_observable()), Some(immediate()));
    let cmd2 = ReactiveCommand::create_param_with(|x: i32| x, Some(can_execute2.as_observable()), Some(immediate()));

    let combined = ReactiveCommand::create_combined_with([cmd1, cmd2], None, Some(immediate()));

    assert!(!first(&combined.can_execute()));

    can_execute2.on_next(true);
    assert!(first(&combined.can_execute()));
}

#[test]
fn create_combined_executes_all_child_commands() {
    with_immediate_schedulers();
    let executed1 = Rc::new(Cell::new(false));
    let executed2 = Rc::new(Cell::new(false));
    let executed3 = Rc::new(Cell::new(false));
    let (flag1, flag2, flag3) = (executed1.clone(), executed2.clone(), executed3.clone());

    let cmd1 = ReactiveCommand::create_param_with(
        move |x: i32| {
            flag1.set(true);
            x * 2
        },
        None,
        Some(immediate()),
    );
    let cmd2 = ReactiveCommand::create_param_with(
        move |x: i32| {
            flag2.set(true);
            x * 3
        },
        None,
        Some(immediate()),
    );
    let cmd3 = ReactiveCommand::create_param_with(
        move |x: i32| {
            flag3.set(true);
            x * 4
        },
        None,
        Some(immediate()),
    );

    let combined = ReactiveCommand::create_combined_with([cmd1, cmd2, cmd3], None, Some(immediate()));
    let results = collect_items(&combined.as_observable());

    first(&combined.execute(5));

    assert!(executed1.get());
    assert!(executed2.get());
    assert!(executed3.get());
    assert_eq!(*results.borrow(), [vec![10, 15, 20]]);
}

#[test]
fn create_combined_propagates_child_exceptions() {
    with_immediate_schedulers();
    let cmd1 = ReactiveCommand::try_create_param_with(|x: i32| Ok(x), None, Some(immediate()));
    let cmd2 = ReactiveCommand::try_create_param_with(|_x: i32| Err(InvalidOperationException::error("Test exception")), None, Some(immediate()));

    let combined = ReactiveCommand::create_combined_with([cmd1, cmd2], None, Some(immediate()));
    let exceptions = collect_items(&combined.thrown_exceptions());

    let _ = subscribe_ignoring_errors(&combined.execute(5));

    assert_eq!(exceptions.borrow().len(), 1);
    assert!(is_invalid_operation(&exceptions.borrow()[0]));
}

#[test]
#[should_panic(expected = "No child commands provided")]
fn create_combined_throws_on_empty_child_commands() {
    with_immediate_schedulers();
    let _ = ReactiveCommand::create_combined::<i32, i32, ReactiveCommand<i32, i32>>(Vec::new());
}

// Not ported: CreateCombined_ThrowsOnNullChildCommands - argument validation

#[test]
fn create_from_observable_without_param_emits_multiple_values() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_from_observable_with(|| Observable::to_observable(vec![1, 2, 3]), None, Some(immediate()));
    let results = collect_items(&command.as_observable());

    // Upstream awaits `Execute().FirstAsync()` and still sees the three results: the default
    // flavour's `ToObservable` pushes every value inside the subscription. Under System.Reactive
    // the iteration runs on the current-thread trampoline, so `FirstAsync` ends the execution
    // after the first value; awaiting the whole execution keeps what the test checks.
    last(&command.execute_default());

    assert_eq!(*results.borrow(), [1, 2, 3]);
}

#[test]
fn create_from_observable_without_param_emits_observable_results() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_from_observable_with(|| Observable::return_(PARAMETER_VALUE), None, Some(immediate()));
    let results = collect_items(&command.as_observable());

    first(&command.execute_default());

    assert_eq!(results.borrow()[0], PARAMETER_VALUE);
}

// Not ported: CreateFromObservable_WithoutParam_ThrowsOnNullExecute - argument validation

#[test]
fn create_from_observable_with_param_passes_parameter_to_observable() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_from_observable_param_with(|parameter: i32| Observable::return_(parameter.to_string()), None, Some(immediate()));
    let results = collect_items(&command.as_observable());

    first(&command.execute(PARAMETER_VALUE));

    assert_eq!(results.borrow()[0], PARAMETER_VALUE_STRING);
}

// Not ported: CreateFromObservable_WithParam_ThrowsOnNullExecute - argument validation
