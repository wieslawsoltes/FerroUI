//! The binders of commands to the controls of one type: `CreatesCommandBindingViaEvent` and
//! `CreatesCommandBindingViaCommandParameter`.
//!
//! Not an upstream test file: ReactiveUI.Binding writes these bindings at compile time
//! (`DefaultEventBindingPlugin`, `CommandPropertyBindingPlugin`), where the port registers binders;
//! these tests check the affinities and the bindings of the two binders.

use crate::mocks::RecordingCommand;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactiveui::*;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::events::Event;
use ferroui_system::IObserver;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// A control with two events and the properties `Command` and `CommandParameter`.
struct Button {
    click: Event<()>,
    mouse_up: Event<()>,
    command: RefCell<Option<Rc<dyn ICommand>>>,
    command_parameter: RefCell<CommandParameter>,
}

impl Button {
    fn new() -> Rc<Self> {
        Rc::new(Self { click: Event::new(), mouse_up: Event::new(), command: RefCell::new(None), command_parameter: RefCell::new(None) })
    }
}

/// A control of another type.
struct Label;

fn event_binder() -> CreatesCommandBindingViaEvent<Button> {
    CreatesCommandBindingViaEvent::<Button>::new().with_event("MouseUp", |button: &Button| &button.mouse_up).with_event("Click", |button: &Button| &button.click)
}

fn command_parameter_binder() -> CreatesCommandBindingViaCommandParameter<Button> {
    CreatesCommandBindingViaCommandParameter::new(
        PropertyPath::value("Command", |button: &Rc<Button>| button.command.borrow().clone()),
        |button: &Button, command| *button.command.borrow_mut() = command,
        PropertyPath::value("CommandParameter", |button: &Rc<Button>| button.command_parameter.borrow().clone()),
        |button: &Button, parameter| *button.command_parameter.borrow_mut() = parameter,
    )
}

fn parameter(text: &str) -> CommandParameter {
    Some(Rc::new(text.to_owned()) as Rc<dyn Any>)
}

fn target(button: &Rc<Button>) -> Option<Rc<dyn Any>> {
    Some(button.clone())
}

#[test]
fn via_event_affinity_is_default_event_for_the_control_type() {
    let binder = event_binder();

    assert_eq!(binder.get_affinity_for_object(TypeId::of::<Button>(), false), BindingAffinity::DEFAULT_EVENT);
    assert_eq!(binder.get_affinity_for_object(TypeId::of::<Button>(), true), BindingAffinity::EXPLICIT);
    assert_eq!(binder.get_affinity_for_object(TypeId::of::<Label>(), false), 0);
}

#[test]
fn via_event_binds_the_default_event_with_the_latest_parameter() {
    let binder = event_binder();
    let button = Button::new();
    let command = RecordingCommand::new();
    let parameters = Subject::<CommandParameter>::new();

    let binding = binder
        .bind_command_to_object(Some(command.clone()), target(&button), parameters.as_observable())
        .expect("the binder binds a button");

    parameters.on_next(parameter("first"));
    parameters.on_next(parameter("second"));
    button.mouse_up.raise(&());
    assert_eq!(command.execute_count(), 0);

    button.click.raise(&());
    assert_eq!(command.execute_count(), 1);
    assert_eq!(command.last_parameter().as_deref(), Some("second"));

    binding.dispose();
    button.click.raise(&());
    assert_eq!(command.execute_count(), 1);
}

#[test]
fn via_event_binds_a_named_event() {
    let binder = event_binder();
    let button = Button::new();
    let command = RecordingCommand::new();

    let binding = binder
        .bind_command_to_object_event(Some(command.clone()), target(&button), Observable::return_(parameter("value")), "MouseUp")
        .expect("the button has the event");

    button.click.raise(&());
    button.mouse_up.raise(&());

    assert_eq!(command.execute_count(), 1);
    assert_eq!(command.last_parameter().as_deref(), Some("value"));
    binding.dispose();
}

#[test]
fn via_event_unknown_event_or_missing_command_binds_nothing() {
    let binder = event_binder();
    let button = Button::new();

    assert!(binder.bind_command_to_object_event(Some(RecordingCommand::new()), target(&button), Observable::empty(), "KeyDown").is_none());
    assert!(binder.bind_command_to_object(None, target(&button), Observable::empty()).is_none());
    assert!(binder.bind_command_to_object(Some(RecordingCommand::new()), Some(Rc::new(Label) as Rc<dyn Any>), Observable::empty()).is_none());
}

#[test]
fn via_event_command_that_cannot_execute_is_not_executed() {
    let binder = event_binder();
    let button = Button::new();
    let command = RecordingCommand::new();
    command.can_execute_result.set(false);

    let binding = binder.bind_command_to_object(Some(command.clone()), target(&button), Observable::empty()).expect("the binder binds a button");
    button.click.raise(&());

    assert_eq!(command.execute_count(), 0);
    binding.dispose();
}

#[test]
fn via_command_parameter_affinity_is_explicit_without_an_event() {
    let binder = command_parameter_binder();

    assert_eq!(binder.get_affinity_for_object(TypeId::of::<Button>(), false), BindingAffinity::EXPLICIT);
    assert_eq!(binder.get_affinity_for_object(TypeId::of::<Button>(), true), 0);
    assert_eq!(binder.get_affinity_for_object(TypeId::of::<Label>(), false), 0);
}

#[test]
fn via_command_parameter_assigns_and_restores_the_properties() {
    let binder = command_parameter_binder();
    let button = Button::new();
    let original: Rc<dyn ICommand> = RecordingCommand::new();
    *button.command.borrow_mut() = Some(original.clone());
    *button.command_parameter.borrow_mut() = parameter("original");
    let command: Rc<dyn ICommand> = RecordingCommand::new();
    let parameters = Subject::<CommandParameter>::new();

    let binding = binder.bind_command_to_object(Some(command.clone()), target(&button), parameters.as_observable()).expect("the binder binds a button");
    parameters.on_next(parameter("bound"));

    assert!(button.command.borrow().as_ref().is_some_and(|bound| Rc::ptr_eq(bound, &command)));
    assert_eq!(button.command_parameter.borrow().as_ref().and_then(|value| value.downcast_ref::<String>().cloned()).as_deref(), Some("bound"));

    binding.dispose();

    assert!(button.command.borrow().as_ref().is_some_and(|restored| Rc::ptr_eq(restored, &original)));
    assert_eq!(button.command_parameter.borrow().as_ref().and_then(|value| value.downcast_ref::<String>().cloned()).as_deref(), Some("original"));
}
