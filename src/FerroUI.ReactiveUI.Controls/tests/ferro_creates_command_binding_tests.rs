//! `...CreatesCommandBindingTests` (the tests of ReactiveUI.Binding's integration).
//!
//! A control is handed to the binder as the binding of a command hands it: its handle in an
//! `Rc<dyn Any>` ([`control_target`]), and its type as the type of the handle ([`control_type`]).

use super::binding_support::{start_app, text_parameter, EventSource, FormatError, RecordingCommand, TestObject, ValueSource};
use crate::{control_target, control_type, FerroCreatesCommandBinding, ReactiveCommandAdapter};
use ferroui_base::input::{ICommand as FerroCommand, InputElementImpl};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Border, Button, ButtonImpl, ContentControlImpl, ControlImpl};
use ferroui_reactiveui::ICreatesCommandBinding;
use std::rc::Rc;

/// The routed event the event bindings name.
const CLICK_EVENT: &str = "Click";

/// The parameter the command runs with while it can execute.
const FIRST_PARAMETER: &str = "first";

/// The parameter pushed while the command cannot execute.
const SECOND_PARAMETER: &str = "second";

/// A button class that registers no routed events of its own, so its events come from a base class
/// (`DerivedButton`).
#[repr(C)]
struct DerivedButton {
    base: Button,
}

ferro_class!(DerivedButton: Button);
ferro_impl_classes!(
    DerivedButton: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl
);

impl DerivedButton {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Button::construct() })
    }
}

/// The text of the command parameter of `button`, if it is text.
fn command_parameter_text(button: &Button) -> Option<String> {
    button.command_parameter().and_then(|parameter| parameter.downcast_ref::<String>().cloned())
}

/// Raises the `Click` event on `button`.
fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn get_affinity_for_object_with_a_non_input_element_is_zero() {
    let _app = start_app();

    assert_eq!(FerroCreatesCommandBinding::new().get_affinity_for_object(control_type::<TestObject>(), false), 0);
}

#[test]
fn get_affinity_for_object_with_an_event_is_the_event_affinity() {
    let _app = start_app();

    assert_eq!(
        FerroCreatesCommandBinding::new().get_affinity_for_object(control_type::<Border>(), true),
        FerroCreatesCommandBinding::EVENT_TARGET_AFFINITY
    );
}

#[test]
fn get_affinity_for_object_with_a_command_source_is_the_command_source_affinity() {
    let _app = start_app();

    assert_eq!(
        FerroCreatesCommandBinding::new().get_affinity_for_object(control_type::<Button>(), false),
        FerroCreatesCommandBinding::COMMAND_SOURCE_AFFINITY
    );
}

#[test]
fn get_affinity_for_object_with_an_input_element_that_is_not_a_command_source_is_zero() {
    let _app = start_app();

    assert_eq!(FerroCreatesCommandBinding::new().get_affinity_for_object(control_type::<Border>(), false), 0);
}

#[test]
fn bind_command_to_object_with_a_null_target_binds_nothing() {
    let _app = start_app();

    let binding = FerroCreatesCommandBinding::new().bind_command_to_object(Some(RecordingCommand::new().command()), None, ValueSource::new().observable());

    assert!(binding.is_none());
}

#[test]
fn bind_command_to_object_with_a_null_command_and_no_command_source_binds_nothing() {
    let _app = start_app();

    let binding = FerroCreatesCommandBinding::new().bind_command_to_object(None, Some(control_target(&Border::new())), ValueSource::new().observable());

    assert!(binding.is_none());
}

#[test]
#[should_panic(expected = "Target must be an InputElement and implement ICommandSource")]
fn bind_command_to_object_with_a_command_and_no_command_source_throws() {
    let _app = start_app();
    let border = Border::new();

    let _ = FerroCreatesCommandBinding::new().bind_command_to_object(Some(RecordingCommand::new().command()), Some(control_target(&border)), ValueSource::new().observable());
}

#[test]
fn bind_command_to_object_with_a_null_command_clears_the_command() {
    let _app = start_app();
    let button = Button::new();
    button.set_command(Some(ReactiveCommandAdapter::command_of(RecordingCommand::new().command())));

    let binding = FerroCreatesCommandBinding::new().bind_command_to_object(None, Some(control_target(&button)), ValueSource::new().observable());

    assert!(binding.is_some());
    assert!(button.command().is_none());
}

#[test]
fn bind_command_to_object_with_a_command_sets_the_command_and_follows_the_parameter() {
    let _app = start_app();
    let button = Button::new();
    let command = RecordingCommand::new();
    let parameters = ValueSource::new();

    let binding = FerroCreatesCommandBinding::new()
        .bind_command_to_object(Some(command.command()), Some(control_target(&button)), parameters.observable())
        .expect("a binding");
    parameters.push(text_parameter(FIRST_PARAMETER));

    // The command of the button is the adapter of the command: it runs the command.
    let set = button.command().expect("the command of the button");
    FerroCommand::execute(&*set, button.command_parameter().as_ref());
    assert_eq!(command.executed(), vec![Some(FIRST_PARAMETER.to_string())]);
    assert_eq!(command_parameter_text(&button), Some(FIRST_PARAMETER.to_string()));

    binding.dispose();

    assert!(button.command().is_none());
    assert!(!parameters.has_observer());
}

#[test]
fn bind_command_to_object_after_the_command_is_replaced_disposal_leaves_the_replacement() {
    let _app = start_app();
    let button = Button::new();
    let replacement: Rc<dyn FerroCommand> = ReactiveCommandAdapter::command_of(RecordingCommand::new().command());

    let binding = FerroCreatesCommandBinding::new()
        .bind_command_to_object(Some(RecordingCommand::new().command()), Some(control_target(&button)), ValueSource::new().observable())
        .expect("a binding");
    button.set_command(Some(replacement.clone()));
    binding.dispose();

    let current = button.command().expect("the replacement");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&current), Rc::as_ptr(&replacement)));
}

#[test]
fn bind_command_to_object_to_an_event_with_a_null_command_or_target_binds_nothing() {
    let _app = start_app();
    let binder = FerroCreatesCommandBinding::new();

    assert!(binder.bind_command_to_object_event(None, Some(control_target(&Button::new())), ValueSource::new().observable(), CLICK_EVENT).is_none());
    assert!(binder.bind_command_to_object_event(Some(RecordingCommand::new().command()), None, ValueSource::new().observable(), CLICK_EVENT).is_none());
}

#[test]
#[should_panic(expected = "Target must be an InputElement.")]
fn bind_command_to_object_to_an_event_on_a_non_input_element_throws() {
    let _app = start_app();
    let target = TestObject::new();

    let _ = FerroCreatesCommandBinding::new().bind_command_to_object_event(
        Some(RecordingCommand::new().command()),
        Some(control_target(&target)),
        ValueSource::new().observable(),
        CLICK_EVENT,
    );
}

#[test]
#[should_panic(expected = "Routed Event NoSuchEvent not found on Button element.")]
fn bind_command_to_object_to_an_unknown_event_throws() {
    let _app = start_app();
    let button = Button::new();

    let _ = FerroCreatesCommandBinding::new().bind_command_to_object_event(
        Some(RecordingCommand::new().command()),
        Some(control_target(&button)),
        ValueSource::new().observable(),
        "NoSuchEvent",
    );
}

// Not ported: BindCommandToObject_ToAnEventWithOtherArguments_Throws - reflection (the binder of
// the port is not generic over the type of the event arguments, which it matched by reflection)

#[test]
fn bind_command_to_object_to_a_base_types_event_executes_the_command_until_disposed() {
    let _app = start_app();
    // The routed events of a class are registered when their accessor first runs on the thread.
    let _ = Button::click_event();
    let button = DerivedButton::new();
    let command = RecordingCommand::new();
    let parameters = ValueSource::new();

    let binding = FerroCreatesCommandBinding::new()
        .bind_command_to_object_event(Some(command.command()), Some(control_target(&button)), parameters.observable(), CLICK_EVENT)
        .expect("a binding");
    parameters.push(text_parameter(FIRST_PARAMETER));
    click(&button);

    command.set_enabled(false);
    parameters.push(text_parameter(SECOND_PARAMETER));
    let enabled_while_blocked = button.is_enabled();
    click(&button);

    parameters.complete();
    binding.dispose();
    command.set_enabled(true);
    click(&button);

    assert_eq!(command.executed(), vec![Some(FIRST_PARAMETER.to_string())]);
    assert!(!enabled_while_blocked);
    assert!(button.is_enabled());
}

#[test]
#[should_panic(expected = "FormatException")]
fn bind_command_to_object_to_an_event_rethrows_a_parameter_error() {
    let _app = start_app();
    let _ = Button::click_event();
    let parameters = ValueSource::new();
    let _binding = FerroCreatesCommandBinding::new()
        .bind_command_to_object_event(Some(RecordingCommand::new().command()), Some(control_target(&Button::new())), parameters.observable(), CLICK_EVENT)
        .expect("a binding");

    parameters.fail(Rc::new(FormatError));
}

// Not ported: BindCommandToObject_WithHandlers_RejectsNullDelegates - argument validation

#[test]
fn bind_command_to_object_with_handlers_and_a_null_command_or_target_binds_nothing() {
    let binder = FerroCreatesCommandBinding::new();
    let source = EventSource::new();

    assert!(binder
        .bind_command_to_object_with_handlers(None, Some(source.clone()), ValueSource::new().observable(), source.subscription())
        .is_none());
    assert!(binder
        .bind_command_to_object_with_handlers(Some(RecordingCommand::new().command()), None, ValueSource::new().observable(), source.subscription())
        .is_none());
}

#[test]
fn bind_command_to_object_with_handlers_executes_the_command_until_disposed() {
    let source = EventSource::new();
    let command = RecordingCommand::new();
    let parameters = ValueSource::new();

    let binding = FerroCreatesCommandBinding::new()
        .bind_command_to_object_with_handlers(Some(command.command()), Some(source.clone()), parameters.observable(), source.subscription())
        .expect("a binding");
    parameters.push(text_parameter(FIRST_PARAMETER));
    source.raise();
    command.set_enabled(false);
    source.raise();
    parameters.complete();
    binding.dispose();

    assert_eq!(command.executed(), vec![Some(FIRST_PARAMETER.to_string())]);
    assert!(!source.has_handlers());
    assert!(!parameters.has_observer());
}

#[test]
#[should_panic(expected = "FormatException")]
fn bind_command_to_object_with_handlers_rethrows_a_parameter_error() {
    let source = EventSource::new();
    let parameters = ValueSource::new();
    let _binding = FerroCreatesCommandBinding::new()
        .bind_command_to_object_with_handlers(Some(RecordingCommand::new().command()), Some(source.clone()), parameters.observable(), source.subscription())
        .expect("a binding");

    parameters.fail(Rc::new(FormatError));
}
