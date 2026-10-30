//! Ported from
//! `tests/Xaml.Behaviors.Interactions.UnitTests/DragAndDrop/DragDropCommandsBehaviorTests.cs` of
//! Xaml.Behaviors.
//!
//! Adapted: the exception the can-execute callback of the reference throws when it is called is a
//! panic of the callback, which the test would report.

use crate::tests::support::{is_text, start_app, text, ObservableCommand};
use crate::DragDropCommandsBehavior;
use ferroui_controls::Border;

#[test]
fn can_execute_command_properties_update_when_commands_raise_can_execute_changed() {
    let _app = start_app();
    let drag_enter_command = ObservableCommand::with_result(false);
    let drag_over_command = ObservableCommand::with_result(false);
    let drag_leave_command = ObservableCommand::with_result(false);
    let drop_command = ObservableCommand::with_result(false);
    let behavior = DragDropCommandsBehavior::new();
    behavior.set_drag_enter_command(drag_enter_command.as_command());
    behavior.set_drag_over_command(drag_over_command.as_command());
    behavior.set_drag_leave_command(drag_leave_command.as_command());
    behavior.set_drop_command(drop_command.as_command());
    behavior.set_pass_event_args_to_command(false);
    let border = Border::new();

    behavior.attach(border.clone());

    assert!(!behavior.can_execute_drag_enter_command());
    assert!(!behavior.can_execute_drag_over_command());
    assert!(!behavior.can_execute_drag_leave_command());
    assert!(!behavior.can_execute_drop_command());

    drag_enter_command.can_execute_result.set(true);
    drag_over_command.can_execute_result.set(true);
    drag_leave_command.can_execute_result.set(true);
    drop_command.can_execute_result.set(true);
    drag_enter_command.raise_can_execute_changed();
    drag_over_command.raise_can_execute_changed();
    drag_leave_command.raise_can_execute_changed();
    drop_command.raise_can_execute_changed();

    assert!(behavior.can_execute_drag_enter_command());
    assert!(behavior.can_execute_drag_over_command());
    assert!(behavior.can_execute_drag_leave_command());
    assert!(behavior.can_execute_drop_command());

    behavior.detach();
}

#[test]
fn can_execute_command_properties_rewire_subscriptions_when_commands_change_and_detach() {
    let _app = start_app();
    let first_drop_command = ObservableCommand::with_result(false);
    let second_drop_command = ObservableCommand::with_result(true);
    let behavior = DragDropCommandsBehavior::new();
    behavior.set_drop_command(first_drop_command.as_command());
    behavior.set_pass_event_args_to_command(false);
    let border = Border::new();

    behavior.attach(border.clone());

    assert_eq!(1, first_drop_command.subscription_count());
    assert!(!behavior.can_execute_drop_command());

    behavior.set_drop_command(second_drop_command.as_command());

    assert_eq!(0, first_drop_command.subscription_count());
    assert_eq!(1, second_drop_command.subscription_count());
    assert!(behavior.can_execute_drop_command());

    behavior.detach();

    assert_eq!(0, second_drop_command.subscription_count());
}

#[test]
fn can_execute_command_properties_use_null_when_event_args_are_not_passed() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|parameter| parameter.is_none());
    let behavior = DragDropCommandsBehavior::new();
    behavior.set_drop_command(command.as_command());
    behavior.set_pass_event_args_to_command(false);
    let border = Border::new();

    behavior.attach(border.clone());

    assert!(behavior.can_execute_drop_command());
    assert!(command.last_can_execute_parameter().is_none());

    behavior.detach();
}

#[test]
fn can_execute_command_properties_defer_when_drag_event_args_are_unavailable() {
    let _app = start_app();
    let command =
        ObservableCommand::with_callback(|_| panic!("The drag event args are not available yet."));
    let behavior = DragDropCommandsBehavior::new();
    behavior.set_drop_command(command.as_command());
    let border = Border::new();

    behavior.attach(border.clone());

    assert!(behavior.can_execute_drop_command());
    assert_eq!(0, command.can_execute_call_count());

    command.raise_can_execute_changed();

    assert!(behavior.can_execute_drop_command());
    assert_eq!(0, command.can_execute_call_count());

    behavior.detach();
}

#[test]
fn can_execute_command_properties_use_can_execute_command_parameter_when_event_args_are_passed() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|parameter| is_text(parameter, "enabled"));
    let behavior = DragDropCommandsBehavior::new();
    behavior.set_drop_command(command.as_command());
    behavior.set_can_execute_command_parameter(Some(text("disabled")));
    let border = Border::new();

    behavior.attach(border.clone());
    assert!(!behavior.can_execute_drop_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "disabled"));

    behavior.set_can_execute_command_parameter(Some(text("enabled")));

    assert!(behavior.can_execute_drop_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "enabled"));

    behavior.detach();
}

#[test]
fn pass_event_args_to_command_updates_can_execute_parameter_availability() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = DragDropCommandsBehavior::new();
    behavior.set_drop_command(command.as_command());
    let border = Border::new();

    behavior.attach(border.clone());
    assert!(behavior.can_execute_drop_command());
    assert_eq!(0, command.can_execute_call_count());

    behavior.set_pass_event_args_to_command(false);

    assert!(!behavior.can_execute_drop_command());
    assert_eq!(1, command.can_execute_call_count());

    behavior.detach();
}
