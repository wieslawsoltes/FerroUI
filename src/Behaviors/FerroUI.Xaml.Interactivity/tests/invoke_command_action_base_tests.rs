//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/InvokeCommandActionBaseTests.cs` of
//! Xaml.Behaviors.

use super::command_can_execute_observer_tests::{is_text, ObservableCommand};
use super::support::{start_app, text};
use crate::{InvokeCommandActionBase, StyledElementActionImpl};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::Border;

/// `TestInvokeCommandAction`.
#[repr(C)]
struct TestInvokeCommandAction {
    base: InvokeCommandActionBase,
}

ferro_class!(TestInvokeCommandAction: InvokeCommandActionBase);
ferroui_base::ferro_impl_classes!(TestInvokeCommandAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for TestInvokeCommandAction {
    fn execute(_this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        None
    }
}

impl TestInvokeCommandAction {
    fn new() -> Ref<Self> {
        instantiate(Self { base: InvokeCommandActionBase::construct() })
    }

    fn resolve_parameter_for_test(&self, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        self.resolve_parameter(parameter)
    }
}

#[test]
fn can_execute_command_defaults_to_true_without_command() {
    let _app = start_app();
    let action = TestInvokeCommandAction::new();
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);

    assert!(action.can_execute_command());

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn can_execute_command_updates_when_command_raises_can_execute_changed() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let action = TestInvokeCommandAction::new();
    action.set_command(command.as_command());
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);
    assert!(!action.can_execute_command());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(action.can_execute_command());

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn use_command_can_execute_for_is_enabled_default_false_does_not_update_parent_is_enabled() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let action = TestInvokeCommandAction::new();
    action.set_command(command.as_command());
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);

    assert!(!action.can_execute_command());
    assert!(parent.is_enabled());

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn use_command_can_execute_for_is_enabled_updates_parent_is_enabled() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let action = TestInvokeCommandAction::new();
    action.set_command(command.as_command());
    action.set_use_command_can_execute_for_is_enabled(true);
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);

    assert!(!parent.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(parent.is_enabled());

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn use_command_can_execute_for_is_enabled_preserves_existing_disabled_parent_state() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let action = TestInvokeCommandAction::new();
    action.set_command(command.as_command());
    action.set_use_command_can_execute_for_is_enabled(true);
    let parent = Border::new();
    parent.set_is_enabled(false);

    action.attach_action_to_logical_tree(&parent);

    assert!(!parent.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(!parent.is_enabled());

    parent.set_is_enabled(true);

    assert!(parent.is_enabled());

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn use_command_can_execute_for_is_enabled_composes_multiple_action_blockers() {
    let _app = start_app();
    let first_command = ObservableCommand::with_result(false);
    let second_command = ObservableCommand::with_result(false);
    let first_action = TestInvokeCommandAction::new();
    first_action.set_command(first_command.as_command());
    first_action.set_use_command_can_execute_for_is_enabled(true);
    let second_action = TestInvokeCommandAction::new();
    second_action.set_command(second_command.as_command());
    second_action.set_use_command_can_execute_for_is_enabled(true);
    let parent = Border::new();

    first_action.attach_action_to_logical_tree(&parent);
    second_action.attach_action_to_logical_tree(&parent);

    assert!(!parent.is_enabled());

    first_command.can_execute_result.set(true);
    first_command.raise_can_execute_changed();

    assert!(!parent.is_enabled());

    second_command.can_execute_result.set(true);
    second_command.raise_can_execute_changed();

    assert!(parent.is_enabled());

    first_action.detach_action_from_logical_tree(&parent);
    second_action.detach_action_from_logical_tree(&parent);
}

#[test]
fn use_command_can_execute_for_is_enabled_property_change_attaches_and_detaches_binding() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let action = TestInvokeCommandAction::new();
    action.set_command(command.as_command());
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);

    assert!(parent.is_enabled());

    action.set_use_command_can_execute_for_is_enabled(true);

    assert!(!parent.is_enabled());

    action.set_use_command_can_execute_for_is_enabled(false);

    assert!(parent.is_enabled());

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn can_execute_command_uses_current_command_parameter() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|parameter| is_text(parameter, "enabled"));
    let action = TestInvokeCommandAction::new();
    action.set_command(command.as_command());
    action.set_command_parameter(Some(text("disabled")));
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);
    assert!(!action.can_execute_command());

    action.set_command_parameter(Some(text("enabled")));

    assert!(action.can_execute_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "enabled"));

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn can_execute_command_defers_when_event_parameter_is_unavailable() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|_| panic!("The event parameter is not available yet."));
    let action = TestInvokeCommandAction::new();
    action.set_command(command.as_command());
    action.set_pass_event_args_to_command(true);
    action.set_use_command_can_execute_for_is_enabled(true);
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);

    assert!(action.can_execute_command());
    assert!(parent.is_enabled());
    assert_eq!(0, command.can_execute_call_count());

    command.raise_can_execute_changed();

    assert!(action.can_execute_command());
    assert!(parent.is_enabled());
    assert_eq!(0, command.can_execute_call_count());

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn can_execute_command_uses_separate_can_execute_command_parameter() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|parameter| is_text(parameter, "enabled"));
    let action = TestInvokeCommandAction::new();
    action.set_command(command.as_command());
    action.set_pass_event_args_to_command(true);
    action.set_can_execute_command_parameter(Some(text("disabled")));
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);
    assert!(!action.can_execute_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "disabled"));

    action.set_can_execute_command_parameter(Some(text("enabled")));

    assert!(action.can_execute_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "enabled"));
    assert!(is_text(action.resolve_parameter_for_test(Some(&text("event parameter"))).as_ref(), "event parameter"));

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn pass_event_args_to_command_updates_can_execute_parameter_availability() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let action = TestInvokeCommandAction::new();
    action.set_command(command.as_command());
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);
    assert!(!action.can_execute_command());
    assert_eq!(1, command.can_execute_call_count());

    action.set_pass_event_args_to_command(true);

    assert!(action.can_execute_command());
    assert_eq!(1, command.can_execute_call_count());

    action.detach_action_from_logical_tree(&parent);
}

#[test]
fn can_execute_command_rewires_subscriptions_when_command_changes_and_detaches() {
    let _app = start_app();
    let first_command = ObservableCommand::with_result(false);
    let second_command = ObservableCommand::with_result(true);
    let action = TestInvokeCommandAction::new();
    action.set_command(first_command.as_command());
    let parent = Border::new();

    action.attach_action_to_logical_tree(&parent);

    assert_eq!(1, first_command.subscription_count());
    assert!(!action.can_execute_command());

    action.set_command(second_command.as_command());

    assert_eq!(0, first_command.subscription_count());
    assert_eq!(1, second_command.subscription_count());
    assert!(action.can_execute_command());

    action.detach_action_from_logical_tree(&parent);

    assert_eq!(0, second_command.subscription_count());
}
