//! `ReactiveCommandTest` (`ReactiveUI.Tests/Commands/ReactiveCommandTest.InvokeCommand.cs`).
//!
//! A command that throws upstream is a `try_` command whose function returns an `Err`.

use crate::mocks::*;
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObserver};
use std::cell::Cell;
use std::rc::Rc;

const PARAMETER_VALUE: i32 = 42;

#[test]
fn invoke_command_icommand_in_target_respects_can_execute() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let can_execute = BehaviorSubject::new(false);
    let target = CommandHolder::new();
    let source = Subject::<()>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(CommandHolder => the_command));
    let flag = executed.clone();
    target.set_the_command(Some(Rc::new(ReactiveCommand::create_with(move || flag.set(true), Some(can_execute.as_observable()), Some(immediate())))));

    source.on_next(());
    assert!(!executed.get());

    can_execute.on_next(true);
    source.on_next(());
    assert!(executed.get());
}

#[test]
fn invoke_command_icommand_in_target_respects_can_execute_window() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let can_execute = BehaviorSubject::new(false);
    let target = CommandHolder::new();
    let source = Subject::<()>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(CommandHolder => the_command));
    let flag = executed.clone();
    target.set_the_command(Some(Rc::new(ReactiveCommand::create_with(move || flag.set(true), Some(can_execute.as_observable()), Some(immediate())))));

    source.on_next(());
    assert!(!executed.get());

    // When the window reopens, the previous requests do not execute.
    can_execute.on_next(true);
    assert!(!executed.get());
}

#[test]
fn invoke_command_icommand_in_target_swallows_exceptions() {
    with_immediate_schedulers();
    let count = Rc::new(Cell::new(0));
    let target = CommandHolder::new();
    let counter = count.clone();
    let command = ReactiveCommand::try_create_with(
        move || {
            counter.set(counter.get() + 1);
            Err::<(), _>(InvalidOperationException::error(""))
        },
        None,
        Some(immediate()),
    );
    let _ = command.thrown_exceptions().subscribe_next(|_| {});
    target.set_the_command(Some(Rc::new(command)));
    let source = Subject::<()>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(CommandHolder => the_command));

    source.on_next(());
    source.on_next(());

    assert_eq!(count.get(), 2);
}

#[test]
fn invoke_command_reactive_command_invokes_command() {
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
    let source = Subject::<()>::new();
    let _ = source.as_observable().invoke_command(command);

    source.on_next(());
    assert_eq!(execution_count.get(), 1);

    source.on_next(());
    assert_eq!(execution_count.get(), 2);
}

#[test]
fn invoke_command_reactive_command_passes_parameter() {
    with_immediate_schedulers();
    let received_params = Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = received_params.clone();
    let command = ReactiveCommand::create_param_with(move |parameter: i32| sink.borrow_mut().push(parameter), None, Some(immediate()));
    let source = Subject::<i32>::new();
    let _ = source.as_observable().invoke_command(command);

    source.on_next(PARAMETER_VALUE);
    source.on_next(100);

    assert_eq!(*received_params.borrow(), [PARAMETER_VALUE, 100]);
}

#[test]
fn invoke_command_reactive_command_respects_can_execute() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let can_execute = BehaviorSubject::new(false);
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

#[test]
fn invoke_command_reactive_command_respects_can_execute_window() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let can_execute = BehaviorSubject::new(false);
    let flag = executed.clone();
    let command = ReactiveCommand::create_with(move || flag.set(true), Some(can_execute.as_observable()), Some(immediate()));
    let source = Subject::<()>::new();
    let _ = source.as_observable().invoke_command(command);

    source.on_next(());
    assert!(!executed.get());

    // When the window reopens, the previous requests do not execute.
    can_execute.on_next(true);
    assert!(!executed.get());
}

#[test]
fn invoke_command_reactive_command_swallows_exceptions() {
    with_immediate_schedulers();
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    let command = ReactiveCommand::try_create_with(
        move || {
            counter.set(counter.get() + 1);
            Err::<(), _>(InvalidOperationException::error(""))
        },
        None,
        Some(immediate()),
    );
    let _ = command.thrown_exceptions().subscribe_next(|_| {});
    let source = Subject::<()>::new();
    let _ = source.as_observable().invoke_command(command);

    source.on_next(());
    source.on_next(());

    assert_eq!(count.get(), 2);
}

#[test]
fn invoke_command_reactive_command_in_target_invokes_command() {
    with_immediate_schedulers();
    let execution_count = Rc::new(Cell::new(0));
    let target = ReactiveCommandHolder::new();
    let source = Subject::<i32>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(ReactiveCommandHolder => the_command));
    let count = execution_count.clone();
    target.set_the_command(Some(ReactiveCommand::create_param_with(move |_: i32| count.set(count.get() + 1), None, Some(immediate()))));

    source.on_next(0);
    assert_eq!(execution_count.get(), 1);

    source.on_next(0);
    assert_eq!(execution_count.get(), 2);
}

#[test]
fn invoke_command_reactive_command_in_target_passes_parameter() {
    with_immediate_schedulers();
    let received_param = Rc::new(Cell::new(0));
    let target = ReactiveCommandHolder::new();
    let source = Subject::<i32>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(ReactiveCommandHolder => the_command));
    let param = received_param.clone();
    target.set_the_command(Some(ReactiveCommand::create_param_with(move |parameter: i32| param.set(parameter), None, Some(immediate()))));

    source.on_next(PARAMETER_VALUE);

    assert_eq!(received_param.get(), PARAMETER_VALUE);
}

#[test]
fn invoke_command_reactive_command_in_target_respects_can_execute() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let can_execute = BehaviorSubject::new(false);
    let target = ReactiveCommandHolder::new();
    let source = Subject::<i32>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(ReactiveCommandHolder => the_command));
    let flag = executed.clone();
    target.set_the_command(Some(ReactiveCommand::create_param_with(move |_: i32| flag.set(true), Some(can_execute.as_observable()), Some(immediate()))));

    source.on_next(0);
    assert!(!executed.get());

    can_execute.on_next(true);
    source.on_next(0);
    assert!(executed.get());
}

#[test]
fn invoke_command_reactive_command_in_target_respects_can_execute_window() {
    with_immediate_schedulers();
    let executed = Rc::new(Cell::new(false));
    let can_execute = BehaviorSubject::new(false);
    let target = ReactiveCommandHolder::new();
    let source = Subject::<i32>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(ReactiveCommandHolder => the_command));
    let flag = executed.clone();
    target.set_the_command(Some(ReactiveCommand::create_param_with(move |_: i32| flag.set(true), Some(can_execute.as_observable()), Some(immediate()))));

    source.on_next(0);
    assert!(!executed.get());

    // When the window reopens, the previous requests do not execute.
    can_execute.on_next(true);
    assert!(!executed.get());
}

#[test]
fn invoke_command_reactive_command_in_target_swallows_exceptions() {
    with_immediate_schedulers();
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    let command = ReactiveCommand::try_create_param_with(
        move |_: i32| {
            counter.set(counter.get() + 1);
            Err::<(), _>(InvalidOperationException::error(""))
        },
        None,
        Some(immediate()),
    );
    let target = ReactiveCommandHolder::new();
    target.set_the_command(Some(command));
    let _ = target.the_command().expect("the command is set").thrown_exceptions().subscribe_next(|_| {});
    let source = Subject::<i32>::new();
    let _ = source.as_observable().invoke_command_path(Some(target.clone()), prop!(ReactiveCommandHolder => the_command));

    source.on_next(0);
    source.on_next(0);

    assert_eq!(count.get(), 2);
}
