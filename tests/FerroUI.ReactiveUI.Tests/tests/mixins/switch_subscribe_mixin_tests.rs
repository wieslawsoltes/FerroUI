//! `SwitchSubscribeMixinTests` (`ReactiveUI.Tests/Mixins/SwitchSubscribeMixinTests.cs`).
//!
//! The overloads of upstream are the methods of `SwitchSubscribeMixins`: `switch_subscribe` and
//! `switch_subscribe_all` on observables of observables and of commands, `switch_select` and
//! `switch_subscribe_with*` with a selector. The tests of the null checks of the arguments are not
//! ported: the arguments cannot be null.

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const DOUBLE_MULTIPLIER: i32 = 2;
const TRIPLE_MULTIPLIER: i32 = 3;
const COMMAND_INPUT: i32 = 5;

/// A command of the tests, as the observables of commands carry it.
type Command = Rc<dyn IReactiveCommandOf<i32, i32>>;

/// A view model holding an observable (`TestViewModel`).
struct TestViewModel {
    observable: Rc<dyn IObservable<i32>>,
}

fn view_model(observable: Rc<dyn IObservable<i32>>) -> Option<Rc<TestViewModel>> {
    Some(Rc::new(TestViewModel { observable }))
}

fn doubling_command() -> ReactiveCommand<i32, i32> {
    ReactiveCommand::create_param_with(|x: i32| x * DOUBLE_MULTIPLIER, None, Some(immediate()))
}

fn tripling_command() -> ReactiveCommand<i32, i32> {
    ReactiveCommand::create_param_with(|x: i32| x * TRIPLE_MULTIPLIER, None, Some(immediate()))
}

fn as_command(command: &ReactiveCommand<i32, i32>) -> Option<Command> {
    Some(Rc::new(command.clone()))
}

fn recorder<T: 'static>() -> (Rc<RefCell<Vec<T>>>, impl Fn(T) + 'static) {
    let values = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    (values, move |value| sink.borrow_mut().push(value))
}

// Not ported: SwitchSubscribe_Observable_ShouldThrowArgumentNullException_WhenSourceIsNull - argument validation
// Not ported: SwitchSubscribe_Observable_ShouldThrowArgumentNullException_WhenOnNextIsNull - argument validation

#[test]
fn switch_subscribe_observable_should_receive_values() {
    let inner = BehaviorSubject::new(1);
    let outer = BehaviorSubject::<Option<Rc<dyn IObservable<i32>>>>::new(Some(inner.as_observable()));
    let (values, on_next) = recorder::<i32>();

    let subscription = outer.as_observable().switch_subscribe(on_next);

    assert_eq!(*values.borrow(), [1]);
    subscription.dispose();
}

#[test]
fn switch_subscribe_observable_should_switch_to_new_observable() {
    const SECOND_INNER_VALUE: i32 = 10;
    let inner1 = BehaviorSubject::new(1);
    let inner2 = BehaviorSubject::new(SECOND_INNER_VALUE);
    let outer = BehaviorSubject::<Option<Rc<dyn IObservable<i32>>>>::new(Some(inner1.as_observable()));
    let (values, on_next) = recorder::<i32>();

    let subscription = outer.as_observable().switch_subscribe(on_next);

    // Switch to the new observable.
    outer.on_next(Some(inner2.as_observable()));

    assert_eq!(*values.borrow(), [1, SECOND_INNER_VALUE]);
    subscription.dispose();
}

#[test]
fn switch_subscribe_observable_should_ignore_null_observables() {
    let inner = BehaviorSubject::new(1);
    let outer = BehaviorSubject::<Option<Rc<dyn IObservable<i32>>>>::new(Some(inner.as_observable()));
    let (values, on_next) = recorder::<i32>();

    let subscription = outer.as_observable().switch_subscribe(on_next);

    outer.on_next(None);

    assert_eq!(*values.borrow(), [1]);
    subscription.dispose();
}

// Not ported: SwitchSubscribe_WithHandlers_ShouldThrowArgumentNullException_WhenAnyParameterIsNull - argument validation

#[test]
fn switch_subscribe_with_handlers_should_call_on_error() {
    let inner = Subject::<i32>::new();
    let inner_observable: Rc<dyn IObservable<i32>> = inner.as_observable();
    let outer = Observable::return_(Some(inner_observable));
    let captured_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let captured = captured_error.clone();

    let subscription = outer.switch_subscribe_all(|_: i32| {}, move |error| *captured.borrow_mut() = Some(error), || {});
    inner.on_error(InvalidOperationException::error("test error"));

    assert!(captured_error.borrow().as_ref().is_some_and(is_invalid_operation));
    subscription.dispose();
}

#[test]
fn switch_subscribe_with_handlers_should_call_on_completed() {
    let inner = Subject::<i32>::new();
    let outer = BehaviorSubject::<Option<Rc<dyn IObservable<i32>>>>::new(Some(inner.as_observable()));
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();

    let subscription = outer.as_observable().switch_subscribe_all(|_: i32| {}, |_| {}, move || flag.set(true));
    inner.on_completed(); // Complete the inner observable first.
    outer.on_completed(); // Then complete the outer observable.

    assert!(completed.get());
    subscription.dispose();
}

// Not ported: SwitchSelect_ShouldThrowArgumentNullException_WhenSourceIsNull - argument validation
// Not ported: SwitchSelect_ShouldThrowArgumentNullException_WhenSelectorIsNull - argument validation

#[test]
fn switch_select_should_project_and_switch() {
    const SECOND_SUBJECT_VALUE: i32 = 10;
    let subject1 = BehaviorSubject::new(1);
    let subject2 = BehaviorSubject::new(SECOND_SUBJECT_VALUE);
    let outer = BehaviorSubject::new(view_model(subject1.as_observable()));
    let (values, on_next) = recorder::<i32>();

    let subscription = outer.as_observable().switch_select(|vm: Rc<TestViewModel>| vm.observable.clone()).subscribe_next(on_next);

    outer.on_next(view_model(subject2.as_observable()));

    assert_eq!(*values.borrow(), [1, SECOND_SUBJECT_VALUE]);
    subscription.dispose();
}

#[test]
fn switch_select_should_ignore_null_values() {
    let subject = BehaviorSubject::new(1);
    let outer = BehaviorSubject::new(view_model(subject.as_observable()));
    let (values, on_next) = recorder::<i32>();

    let subscription = outer.as_observable().switch_select(|vm: Rc<TestViewModel>| vm.observable.clone()).subscribe_next(on_next);

    outer.on_next(None);

    assert_eq!(*values.borrow(), [1]);
    subscription.dispose();
}

// Not ported: SwitchSubscribe_WithSelector_ShouldThrowArgumentNullException_WhenAnyParameterIsNull - argument validation

#[test]
fn switch_subscribe_with_selector_should_project_and_subscribe() {
    let subject = BehaviorSubject::new(1);
    let outer = BehaviorSubject::new(view_model(subject.as_observable()));
    let (values, on_next) = recorder::<i32>();

    let subscription = outer.as_observable().switch_subscribe_with(|vm: Rc<TestViewModel>| vm.observable.clone(), on_next);
    subject.on_next(2);

    assert_eq!(*values.borrow(), [1, 2]);
    subscription.dispose();
}

// Not ported: SwitchSubscribe_WithSelectorAndHandlers_ShouldThrowArgumentNullException_WhenAnyParameterIsNull - argument validation
// Not ported: SwitchSubscribe_Command_ShouldThrowArgumentNullException_WhenSourceIsNull - argument validation
// Not ported: SwitchSubscribe_Command_ShouldThrowArgumentNullException_WhenOnNextIsNull - argument validation

#[test]
fn switch_subscribe_command_should_receive_command_results() {
    with_immediate_schedulers();
    let command = doubling_command();
    let outer = BehaviorSubject::new(as_command(&command));
    let (results, on_next) = recorder::<i32>();

    let subscription = outer.as_observable().switch_subscribe(on_next);
    last(&command.execute(COMMAND_INPUT));

    assert_eq!(*results.borrow(), [COMMAND_INPUT * DOUBLE_MULTIPLIER]);
    subscription.dispose();
}

#[test]
fn switch_subscribe_command_should_switch_to_new_command() {
    with_immediate_schedulers();
    let command1 = doubling_command();
    let command2 = tripling_command();
    let outer = BehaviorSubject::new(as_command(&command1));
    let (results, on_next) = recorder::<i32>();

    let subscription = outer.as_observable().switch_subscribe(on_next);
    last(&command1.execute(COMMAND_INPUT));

    // Switch to the new command.
    outer.on_next(as_command(&command2));
    last(&command2.execute(COMMAND_INPUT));

    assert_eq!(*results.borrow(), [COMMAND_INPUT * DOUBLE_MULTIPLIER, COMMAND_INPUT * TRIPLE_MULTIPLIER]);
    subscription.dispose();
}

#[test]
fn switch_subscribe_command_should_ignore_null_commands() {
    with_immediate_schedulers();
    let command = doubling_command();
    let outer = BehaviorSubject::new(as_command(&command));
    let (results, on_next) = recorder::<i32>();

    let subscription = outer.as_observable().switch_subscribe(on_next);
    last(&command.execute(COMMAND_INPUT));

    outer.on_next(None);

    assert_eq!(*results.borrow(), [COMMAND_INPUT * DOUBLE_MULTIPLIER]);
    subscription.dispose();
}

// Not ported: SwitchSubscribe_CommandWithHandlers_ShouldThrowArgumentNullException_WhenAnyParameterIsNull - argument validation
// Not ported: SwitchSelect_Command_ShouldThrowArgumentNullException_WhenSourceIsNull - argument validation
// Not ported: SwitchSelect_Command_ShouldThrowArgumentNullException_WhenSelectorIsNull - argument validation

#[test]
fn switch_select_command_should_project_command_property() {
    with_immediate_schedulers();
    let command = doubling_command();
    let outer = BehaviorSubject::new(as_command(&command));
    let (is_executing_values, on_next) = recorder::<bool>();

    let subscription = outer.as_observable().switch_select(|cmd: Command| cmd.is_executing()).subscribe_next(on_next);

    last(&command.execute(COMMAND_INPUT));

    // At least the initial false value.
    assert!(!is_executing_values.borrow().is_empty());
    assert!(!is_executing_values.borrow()[0]);
    subscription.dispose();
}

#[test]
fn switch_select_command_should_switch_to_new_command_property() {
    with_immediate_schedulers();
    let command1 = doubling_command();
    let command2 = tripling_command();
    let outer = BehaviorSubject::new(as_command(&command1));
    let (can_execute_values, on_next) = recorder::<bool>();

    let subscription = outer.as_observable().switch_select(|cmd: Command| cmd.can_execute()).subscribe_next(on_next);

    // Switch to the new command.
    outer.on_next(as_command(&command2));

    // The initial values of both commands.
    assert!(can_execute_values.borrow().len() >= 2);
    subscription.dispose();
}

// Not ported: SwitchSubscribe_CommandWithSelector_ShouldThrowArgumentNullException_WhenAnyParameterIsNull - argument validation

#[test]
fn switch_subscribe_command_with_selector_should_project_and_subscribe() {
    with_immediate_schedulers();
    let command = doubling_command();
    let outer = BehaviorSubject::new(as_command(&command));
    let (is_executing_values, on_next) = recorder::<bool>();

    let subscription = outer.as_observable().switch_subscribe_with(|cmd: Command| cmd.is_executing(), on_next);
    last(&command.execute(COMMAND_INPUT));

    assert!(!is_executing_values.borrow().is_empty());
    subscription.dispose();
}

// Not ported: SwitchSubscribe_CommandWithSelectorAndHandlers_ShouldThrowArgumentNullException_WhenAnyParameterIsNull - argument validation
