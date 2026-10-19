//! `CommandInvokerTests` (ReactiveUI.Binding.Tests, `CommandBinding/CommandInvokerTests.cs`).
//!
//! The overload on an observable of commands is `CommandInvoker::invoke_observed`. A fault of a
//! sequence is rethrown where it is raised: here the `on_error` of the observer panics.

use super::super::test_models::ManualObservable;
use crate::mocks::*;
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactiveui::*;
use ferroui_system::{IObservable, IObserver};
use std::rc::Rc;

const FIRST_VALUE: &str = "first";
const SECOND_VALUE: &str = "second";
const FAULT_MESSAGE: &str = "faulted";

/// A manual observable as the handle the invoker takes.
fn observable_of<T: 'static>(manual: &Rc<ManualObservable<T>>) -> Rc<dyn IObservable<T>> {
    manual.clone()
}

/// An observable of commands, as `invoke_observed` takes it.
fn commands_of(subject: &Rc<Subject<Option<Rc<dyn ICommand>>>>) -> Rc<dyn IObservable<Option<Rc<dyn ICommand>>>> {
    subject.as_observable()
}

#[test]
fn invoke_fixed_command_executes_with_each_value() {
    let command = RecordingCommand::new();
    let values = Subject::<String>::new();

    let invocation = CommandInvoker::invoke(&values.as_observable(), command.clone());

    values.on_next(FIRST_VALUE.to_owned());
    values.on_next(SECOND_VALUE.to_owned());

    assert_eq!(command.execute_count(), 2);
    assert_eq!(command.last_parameter().as_deref(), Some(SECOND_VALUE));
    invocation.dispose();
}

#[test]
fn invoke_fixed_command_refused_value_is_dropped() {
    let command = RecordingCommand::new();
    command.can_execute_result.set(false);
    let values = Subject::<String>::new();

    let invocation = CommandInvoker::invoke(&values.as_observable(), command.clone());

    values.on_next(FIRST_VALUE.to_owned());

    assert_eq!(command.execute_count(), 0);
    assert_eq!(command.last_questioned_parameter().as_deref(), Some(FIRST_VALUE));

    command.can_execute_result.set(true);
    values.on_next(SECOND_VALUE.to_owned());

    assert_eq!(command.execute_count(), 1);
    invocation.dispose();
}

#[test]
fn invoke_fixed_command_disposed_stops_executing() {
    let command = RecordingCommand::new();
    let values = Subject::<String>::new();

    CommandInvoker::invoke(&values.as_observable(), command.clone()).dispose();

    values.on_next(FIRST_VALUE.to_owned());

    assert_eq!(command.execute_count(), 0);
}

#[test]
fn invoke_observed_command_executes_the_latest_command() {
    let first = RecordingCommand::new();
    let second = RecordingCommand::new();
    let commands = Subject::<Option<Rc<dyn ICommand>>>::new();
    let values = Subject::<String>::new();

    let invocation = CommandInvoker::invoke_observed(&values.as_observable(), &commands_of(&commands));

    commands.on_next(Some(first.clone()));
    values.on_next(FIRST_VALUE.to_owned());
    commands.on_next(Some(second.clone()));
    values.on_next(SECOND_VALUE.to_owned());

    assert_eq!(first.execute_count(), 1);
    assert_eq!(second.execute_count(), 1);
    assert_eq!(second.last_parameter().as_deref(), Some(SECOND_VALUE));
    invocation.dispose();
}

#[test]
fn invoke_observed_command_no_command_yet_drops_the_values() {
    let command = RecordingCommand::new();
    let commands = Subject::<Option<Rc<dyn ICommand>>>::new();
    let values = Subject::<String>::new();

    let invocation = CommandInvoker::invoke_observed(&values.as_observable(), &commands_of(&commands));

    values.on_next(FIRST_VALUE.to_owned());
    commands.on_next(Some(command.clone()));
    values.on_next(SECOND_VALUE.to_owned());

    assert_eq!(command.execute_count(), 1);
    assert_eq!(command.last_parameter().as_deref(), Some(SECOND_VALUE));
    invocation.dispose();
}

#[test]
fn invoke_observed_command_null_command_drops_the_values() {
    let command = RecordingCommand::new();
    let commands = Subject::<Option<Rc<dyn ICommand>>>::new();
    let values = Subject::<String>::new();

    let invocation = CommandInvoker::invoke_observed(&values.as_observable(), &commands_of(&commands));

    commands.on_next(Some(command.clone()));
    commands.on_next(None);
    values.on_next(FIRST_VALUE.to_owned());

    assert_eq!(command.execute_count(), 0);
    invocation.dispose();
}

#[test]
fn invoke_observed_command_new_command_alone_does_not_execute() {
    let command = RecordingCommand::new();
    let commands = Subject::<Option<Rc<dyn ICommand>>>::new();
    let values = Subject::<String>::new();

    let invocation = CommandInvoker::invoke_observed(&values.as_observable(), &commands_of(&commands));

    values.on_next(FIRST_VALUE.to_owned());
    commands.on_next(Some(command.clone()));

    assert_eq!(command.execute_count(), 0);
    invocation.dispose();
}

#[test]
fn invoke_observed_command_disposed_stops_executing() {
    let command = RecordingCommand::new();
    let commands = Subject::<Option<Rc<dyn ICommand>>>::new();
    let values = Subject::<String>::new();

    CommandInvoker::invoke_observed(&values.as_observable(), &commands_of(&commands)).dispose();

    commands.on_next(Some(command.clone()));
    values.on_next(FIRST_VALUE.to_owned());

    assert_eq!(command.execute_count(), 0);
    assert!(!commands.has_observers());
}

#[test]
fn invoke_values_fault_surfaces_the_fault() {
    let command = RecordingCommand::new();
    let values = ManualObservable::<String>::new();

    let invocation = CommandInvoker::invoke(&observable_of(&values), command);

    let observer = values.observer().expect("the invoker subscribed");
    let message = panic_message(|| observer.on_error(InvalidOperationException::error(FAULT_MESSAGE)));
    assert!(message.contains(FAULT_MESSAGE));
    invocation.dispose();
}

#[test]
fn invoke_observed_command_values_fault_surfaces_the_fault() {
    let commands = Subject::<Option<Rc<dyn ICommand>>>::new();
    let values = ManualObservable::<String>::new();

    let invocation = CommandInvoker::invoke_observed(&observable_of(&values), &commands_of(&commands));

    let observer = values.observer().expect("the invoker subscribed");
    let message = panic_message(|| observer.on_error(InvalidOperationException::error(FAULT_MESSAGE)));
    assert!(message.contains(FAULT_MESSAGE));
    invocation.dispose();
}

#[test]
fn invoke_observed_command_faults_surfaces_the_fault() {
    let commands = ManualObservable::<Option<Rc<dyn ICommand>>>::new();
    let values = Subject::<String>::new();

    let invocation = CommandInvoker::invoke_observed(&values.as_observable(), &observable_of(&commands));

    let observer = commands.observer().expect("the invoker subscribed");
    let message = panic_message(|| observer.on_error(InvalidOperationException::error(FAULT_MESSAGE)));
    assert!(message.contains(FAULT_MESSAGE));
    invocation.dispose();
}

#[test]
fn invoke_fixed_command_values_complete_does_not_throw() {
    let command = RecordingCommand::new();
    let values = ManualObservable::<String>::new();

    let invocation = CommandInvoker::invoke(&observable_of(&values), command.clone());

    values.observer().expect("the invoker subscribed").on_completed();

    assert_eq!(command.execute_count(), 0);
    invocation.dispose();
}

#[test]
fn invoke_sequences_complete_does_not_throw() {
    let command = RecordingCommand::new();
    let commands = ManualObservable::<Option<Rc<dyn ICommand>>>::new();
    let values = ManualObservable::<String>::new();

    let invocation = CommandInvoker::invoke_observed(&observable_of(&values), &observable_of(&commands));

    let commands_observer = commands.observer().expect("the invoker subscribed");
    commands_observer.on_next(Some(command.clone()));
    commands_observer.on_completed();
    values.observer().expect("the invoker subscribed").on_completed();

    assert_eq!(command.execute_count(), 0);
    invocation.dispose();
}

// Not ported: Invoke_NullSource_Throws - argument validation
// Not ported: Invoke_NullCommand_Throws - argument validation
// Not ported: Invoke_NullCommandSequence_Throws - argument validation
// Not ported: Invoke_ObservedCommand_NullSource_Throws - argument validation
