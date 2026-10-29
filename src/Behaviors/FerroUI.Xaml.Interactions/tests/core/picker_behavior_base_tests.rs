//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/PickerBehaviorBaseTests.cs` of
//! Xaml.Behaviors.

use super::observable_command::{is_text, ObservableCommand};
use crate::tests::support::{start_app, text};
use crate::{
    ButtonOpenFilePickerBehavior, ButtonOpenFolderPickerBehavior, ButtonSaveFilePickerBehavior,
    MenuItemSaveFilePickerBehavior,
};
use ferroui_controls::{Button, MenuItem};

#[test]
fn can_execute_command_defaults_to_true_without_command() {
    let _app = start_app();
    let behavior = ButtonOpenFilePickerBehavior::new();
    let button = Button::new();

    behavior.attach(&button);

    assert!(behavior.can_execute_command());

    behavior.detach();
}

#[test]
fn can_execute_command_updates_when_attached() {
    let _app = start_app();
    let behavior = ButtonOpenFilePickerBehavior::new();
    behavior.set_command(ObservableCommand::with_result(false).as_command());
    behavior.set_can_execute_command_parameter(Some(text("probe")));
    let button = Button::new();

    behavior.attach(&button);

    assert!(!behavior.can_execute_command());

    behavior.detach();
}

#[test]
fn can_execute_command_defers_when_picker_result_parameter_is_unavailable() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|_| panic!("The picker result is not available yet."));
    let behavior = ButtonOpenFilePickerBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_use_command_can_execute_for_is_enabled(true);
    let button = Button::new();

    behavior.attach(&button);

    assert!(behavior.can_execute_command());
    assert!(button.is_enabled());
    assert_eq!(0, command.can_execute_call_count());

    command.raise_can_execute_changed();

    assert!(behavior.can_execute_command());
    assert!(button.is_enabled());
    assert_eq!(0, command.can_execute_call_count());

    behavior.detach();
}

#[test]
fn use_command_can_execute_for_is_enabled_updates_button_picker_associated_object() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = ButtonOpenFilePickerBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_can_execute_command_parameter(Some(text("probe")));
    behavior.set_use_command_can_execute_for_is_enabled(true);
    let button = Button::new();

    behavior.attach(&button);

    assert!(!button.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(button.is_enabled());

    behavior.detach();
}

#[test]
fn use_command_can_execute_for_is_enabled_updates_menu_item_picker_associated_object() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = MenuItemSaveFilePickerBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_can_execute_command_parameter(Some(text("probe")));
    behavior.set_use_command_can_execute_for_is_enabled(true);
    let menu_item = MenuItem::new();

    behavior.attach(&menu_item);

    assert!(!menu_item.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(menu_item.is_enabled());

    behavior.detach();
}

#[test]
fn can_execute_command_updates_when_command_raises_can_execute_changed() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = ButtonOpenFilePickerBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_can_execute_command_parameter(Some(text("probe")));
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
    let behavior = ButtonOpenFolderPickerBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_command_parameter(Some(text("disabled")));
    let button = Button::new();

    behavior.attach(&button);
    assert!(!behavior.can_execute_command());

    behavior.set_command_parameter(Some(text("enabled")));

    assert!(behavior.can_execute_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "enabled"));

    behavior.detach();
}

#[test]
fn can_execute_command_rewires_subscriptions_when_command_changes_and_detaches() {
    let _app = start_app();
    let first_command = ObservableCommand::with_result(false);
    let second_command = ObservableCommand::with_result(true);
    let behavior = ButtonSaveFilePickerBehavior::new();
    behavior.set_command(first_command.as_command());
    behavior.set_can_execute_command_parameter(Some(text("probe")));
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
