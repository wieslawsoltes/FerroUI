//! `ReactiveCommandTest` (`ReactiveUI.Tests/Commands/ReactiveCommandTest.ExecutionAndInvoke.cs`).
//!
//! A command used as `ICommand` is a `Rc<dyn ICommand>`, whose parameter is `Option<&dyn Any>`;
//! `InvokeCommand(target, x => x.TheCommand)` is `invoke_command_path` with a property path.

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const PARAMETER_VALUE: i32 = 42;
const TEST_ERROR_MESSAGE: &str = "Test error";

#[test]
fn execute_can_be_cancelled() {
    let scheduler = TestScheduler::new();
    with_schedulers(scheduler.scheduler());
    let execute = Observable::return_(()).delay_on(Duration::from_secs(1), scheduler.scheduler());
    let command = ReactiveCommand::create_from_observable_with(move || execute.clone(), None, Some(scheduler.scheduler()));
    let executed = collect_items(&command.as_observable());

    let sub1 = command.execute_default().subscribe_ignore();
    let _ = command.execute_default().subscribe_ignore();
    scheduler.advance_by(duration_to_ticks(Duration::from_millis(1)));

    assert!(first(&command.is_executing()));
    assert!(executed.borrow().is_empty());

    sub1.dispose();
    scheduler.advance_by(duration_to_ticks(Duration::from_secs(1)));

    assert_eq!(executed.borrow().len(), 1);
    assert!(!first(&command.is_executing()));
}

#[test]
fn execute_lazy_evaluation() {
    with_immediate_schedulers();
    let execution_count = Rc::new(Cell::new(0));
    let count = execution_count.clone();
    let command = ReactiveCommand::create_with(
        move || {
            count.set(count.get() + 1);
            count.get()
        },
        None,
        Some(immediate()),
    );

    let execution = command.execute_default();
    assert_eq!(execution_count.get(), 0);

    let _ = execution.subscribe_ignore();
    assert_eq!(execution_count.get(), 1);
}

#[test]
fn execute_passes_parameters() {
    with_immediate_schedulers();
    let parameters = Rc::new(RefCell::new(Vec::new()));
    let sink = parameters.clone();
    let command = ReactiveCommand::create_param_with(move |parameter: i32| sink.borrow_mut().push(parameter), None, Some(immediate()));

    first(&command.execute(1));
    first(&command.execute(PARAMETER_VALUE));
    first(&command.execute(348));

    assert_eq!(*parameters.borrow(), [1, PARAMETER_VALUE, 348]);
}

#[test]
fn execute_reenables_after_completion() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_with(|| {}, None, Some(immediate()));
    let can_execute = collect_items(&command.can_execute());

    first(&command.execute_default());

    assert_eq!(*can_execute.borrow(), [true, false, true]);
}

#[test]
fn execute_reenables_after_failure() {
    with_immediate_schedulers();
    let command = ReactiveCommand::create_from_observable_with(
        || Observable::throw::<()>(InvalidOperationException::error(TEST_ERROR_MESSAGE)),
        None,
        Some(immediate()),
    );
    let can_execute = collect_items(&command.can_execute());
    let _ = command.thrown_exceptions().subscribe_next(|_| {});

    let _ = subscribe_ignoring_errors(&command.execute_default());

    assert_eq!(*can_execute.borrow(), [true, false, true]);
}

#[test]
fn execute_ticks_multiple_results() {
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
fn icommand_can_execute_is_false_while_executing() {
    let scheduler = TestScheduler::new();
    with_schedulers(scheduler.scheduler());
    let execute = Observable::return_(()).delay_on(Duration::from_secs(1), scheduler.scheduler());
    let command: Rc<dyn ICommand> =
        Rc::new(ReactiveCommand::create_from_observable_with(move || execute.clone(), None, Some(scheduler.scheduler())));

    assert!(command.can_execute(None));

    command.execute(None);
    scheduler.advance_by(duration_to_ticks(Duration::from_millis(1)));
    assert!(!command.can_execute(None));
}

#[test]
fn icommand_can_execute_returns_correct_value() {
    with_immediate_schedulers();
    let can_execute_subject = BehaviorSubject::new(false);
    let command: Rc<dyn ICommand> = Rc::new(ReactiveCommand::create_with(|| {}, Some(can_execute_subject.as_observable()), Some(immediate())));

    assert!(!command.can_execute(None));

    can_execute_subject.on_next(true);
    assert!(command.can_execute(None));
}

#[test]
fn icommand_can_execute_changed_raises_events() {
    with_immediate_schedulers();
    let can_execute_subject = BehaviorSubject::new(false);
    let command: Rc<dyn ICommand> = Rc::new(ReactiveCommand::create_with(|| {}, Some(can_execute_subject.as_observable()), Some(immediate())));
    let can_execute_changed = Rc::new(RefCell::new(Vec::new()));
    let (sink, sender) = (can_execute_changed.clone(), Rc::downgrade(&command));
    let _handler = command.can_execute_changed(Rc::new(move || {
        if let Some(command) = sender.upgrade() {
            sink.borrow_mut().push(command.can_execute(None));
        }
    }));

    can_execute_subject.on_next(true);
    can_execute_subject.on_next(false);

    assert_eq!(*can_execute_changed.borrow(), [true, false]);
}

#[test]
fn icommand_execute_invokes_command() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let command: Rc<dyn ICommand> = Rc::new(ReactiveCommand::create_with(move || flag.set(true), None, Some(immediate())));

    command.execute(None);

    assert!(executed.get());
}

#[test]
fn icommand_execute_passes_parameter() {
    with_immediate_schedulers();
    let received_param = Rc::new(Cell::new(0));
    let param = received_param.clone();
    let command: Rc<dyn ICommand> = Rc::new(ReactiveCommand::create_param_with(move |parameter: i32| param.set(parameter), None, Some(immediate())));

    command.execute(Some(&PARAMETER_VALUE));

    assert_eq!(received_param.get(), PARAMETER_VALUE);
}

/// Upstream's message names both types (`System.Int32`, `System.String`); the received type is
/// not known here (`dyn Any` has no type name), so the message names the required one.
#[test]
fn icommand_execute_throws_on_incorrect_parameter_type() {
    with_immediate_schedulers();
    let command: Rc<dyn ICommand> = Rc::new(ReactiveCommand::create_param_with(|_: i32| {}, None, Some(immediate())));

    let message = panic_message(|| command.execute(Some(&"wrong type")));

    assert!(message.contains("InvalidOperationException"));
    assert!(message.contains("i32"));
}

/// A nullable parameter is an `Option`: `Execute(42)` passes `Some(42)`, and `Execute(null)` passes
/// no parameter, which is the default `None`.
#[test]
fn icommand_execute_works_with_nullable_parameters() {
    with_immediate_schedulers();
    let received_value: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
    let received = received_value.clone();
    let command: Rc<dyn ICommand> =
        Rc::new(ReactiveCommand::create_param_with(move |parameter: Option<i32>| received.set(parameter), None, Some(immediate())));

    command.execute(Some(&Some(PARAMETER_VALUE)));
    assert_eq!(received_value.get(), Some(PARAMETER_VALUE));

    command.execute(None);
    assert_eq!(received_value.get(), None);
}

#[test]
fn invoke_command_icommand_invokes_command() {
    with_immediate_schedulers();
    let execution_count = Rc::new(Cell::new(0));
    let count = execution_count.clone();
    let command: Rc<dyn ICommand> = Rc::new(ReactiveCommand::create_with(
        move || {
            count.set(count.get() + 1);
            count.get()
        },
        None,
        Some(immediate()),
    ));
    let source = Subject::<()>::new();
    let _ = source.as_observable().invoke_command(command);

    source.on_next(());
    assert_eq!(execution_count.get(), 1);

    source.on_next(());
    assert_eq!(execution_count.get(), 2);
}

#[test]
fn invoke_command_icommand_passes_parameter() {
    with_immediate_schedulers();
    let received_params = Rc::new(RefCell::new(Vec::new()));
    let sink = received_params.clone();
    let command: Rc<dyn ICommand> =
        Rc::new(ReactiveCommand::create_param_with(move |parameter: i32| sink.borrow_mut().push(parameter), None, Some(immediate())));
    let source = Subject::<i32>::new();
    let _ = source.as_observable().invoke_command(command);

    source.on_next(PARAMETER_VALUE);
    source.on_next(100);

    assert_eq!(*received_params.borrow(), [PARAMETER_VALUE, 100]);
}

#[test]
fn invoke_command_icommand_respects_can_execute() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let flag = executed.clone();
    let can_execute = BehaviorSubject::new(false);
    let command: Rc<dyn ICommand> = Rc::new(ReactiveCommand::create_with(move || flag.set(true), Some(can_execute.as_observable()), Some(immediate())));
    let source = Subject::<()>::new();
    let _ = source.as_observable().invoke_command(command);

    source.on_next(());
    assert!(!executed.get());

    can_execute.on_next(true);
    source.on_next(());
    assert!(executed.get());
}

#[test]
fn invoke_command_icommand_works_with_cold_observable() {
    with_immediate_schedulers();
    let execution_count = Rc::new(Cell::new(0));
    let count = execution_count.clone();
    let command: Rc<dyn ICommand> = Rc::new(ReactiveCommand::create_with(
        move || {
            count.set(count.get() + 1);
            count.get()
        },
        None,
        Some(immediate()),
    ));
    let source = Observable::return_(());
    let _ = source.invoke_command(command);

    assert_eq!(execution_count.get(), 1);
}

#[test]
fn invoke_command_icommand_in_target_invokes_command() {
    with_immediate_schedulers();
    let execution_count = Rc::new(Cell::new(0));
    let target = CommandHolder::new();
    let source = Subject::<()>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(CommandHolder => the_command));
    let count = execution_count.clone();
    target.set_the_command(Some(Rc::new(ReactiveCommand::create_with(
        move || {
            count.set(count.get() + 1);
            count.get()
        },
        None,
        Some(immediate()),
    ))));

    source.on_next(());
    assert_eq!(execution_count.get(), 1);

    source.on_next(());
    assert_eq!(execution_count.get(), 2);
}

#[test]
fn invoke_command_icommand_in_target_passes_parameter() {
    with_immediate_schedulers();
    let target = CommandHolder::new();
    let source = Subject::<i32>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(CommandHolder => the_command));
    let command = FakeCommand::<i32>::new();
    target.set_the_command(Some(command.clone()));

    source.on_next(PARAMETER_VALUE);

    assert_eq!(command.can_execute_parameter(), Some(PARAMETER_VALUE));
    assert_eq!(command.execute_parameter(), Some(PARAMETER_VALUE));
}
