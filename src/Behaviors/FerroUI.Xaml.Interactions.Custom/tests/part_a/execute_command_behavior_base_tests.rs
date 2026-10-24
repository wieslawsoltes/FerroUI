//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/ExecuteCommandBehaviorBaseTests.cs`
//! of Xaml.Behaviors.

use super::observable_command::{is_text, ObservableCommand};
use super::support::start_app;
use crate::ExecuteCommandOnTappedBehavior;
use ferroui_base::BoxedValue;
use ferroui_controls::Button;
use std::rc::Rc;

fn text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}

#[test]
fn can_execute_command_updates_when_command_raises_can_execute_changed() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = ExecuteCommandOnTappedBehavior::new();
    behavior.set_command(command.as_command());
    let button = Button::new();

    behavior.attach(&button);
    assert!(!behavior.can_execute_command());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(behavior.can_execute_command());

    behavior.detach();
}

#[test]
fn can_execute_command_uses_current_command_parameter() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|parameter| is_text(parameter, "enabled"));
    let behavior = ExecuteCommandOnTappedBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_command_parameter(text("disabled"));
    let button = Button::new();

    behavior.attach(&button);
    assert!(!behavior.can_execute_command());

    behavior.set_command_parameter(text("enabled"));

    assert!(behavior.can_execute_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "enabled"));

    behavior.detach();
}

#[test]
fn can_execute_command_rewires_subscriptions_when_command_changes_and_detaches() {
    let _app = start_app();
    let first_command = ObservableCommand::with_result(false);
    let second_command = ObservableCommand::with_result(true);
    let behavior = ExecuteCommandOnTappedBehavior::new();
    behavior.set_command(first_command.as_command());
    let button = Button::new();

    behavior.attach(&button);

    assert_eq!(1, first_command.subscription_count());
    assert!(!behavior.can_execute_command());

    behavior.set_command(second_command.as_command());

    assert_eq!(0, first_command.subscription_count());
    assert_eq!(1, second_command.subscription_count());
    assert!(behavior.can_execute_command());

    behavior.detach();

    assert_eq!(0, second_command.subscription_count());
}
