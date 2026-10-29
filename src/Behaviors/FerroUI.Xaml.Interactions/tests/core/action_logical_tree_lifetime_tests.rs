//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/ActionLogicalTreeLifetimeTests.cs`
//! of Xaml.Behaviors.
//!
//! The tests use the click trigger and the conditional and switch actions of the custom behaviors
//! (`ClickEventTrigger`, `ConditionalAction`, `SwitchCaseAction`, `Case`), which the crate
//! `ferroui-xaml-interactions-custom` ports (a dev-dependency of this crate; it does not depend on
//! this crate, so the test build has one copy of each type). The theory of the reference over the kinds of
//! nested action containers is one test per kind.

use super::observable_command::ObservableCommand;
use crate::tests::support::{run_jobs, start_app, text};
use crate::{AsyncActionGroup, DebounceAction, InvokeCommandAction, ThrottleAction};
use ferroui_base::input::InputElement;
use ferroui_base::{FerroObjectExtensions, Ref, StyledElement};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{Border, Button, Control, Grid, StackPanel, Window};
use ferroui_xaml_interactions_custom::{Case, ClickEventTrigger, ConditionalAction, SwitchCaseAction};
use ferroui_xaml_interactivity::{ActionCollection, InvokeCommandActionBase, Interaction, StyledElementAction};
use std::rc::Rc;

/// The kinds of nested action containers (`NestedActionContainerKind`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NestedActionContainerKind {
    AsyncActionGroup,
    DebounceAction,
    ThrottleAction,
    ConditionalActionTrueBranch,
    ConditionalActionFalseBranch,
    SwitchCaseDefaultActions,
    SwitchCaseCaseActions,
}

fn create_window() -> Ref<Window> {
    let window = Window::new();
    window.set_width(320.0);
    window.set_height(200.0);
    window
}

fn create_target() -> Ref<Border> {
    let target = Border::new();
    target.set_width(160.0);
    target.set_height(60.0);
    target.set_focusable(true);
    target
}

fn create_command_action(command: &Rc<ObservableCommand>) -> Ref<InvokeCommandAction> {
    let action = InvokeCommandAction::new();
    action.set_command(command.as_command());
    action.set_can_execute_command_parameter(Some(text("probe")));
    action
}

fn parent_of(element: &StyledElement) -> Option<Ref<StyledElement>> {
    element.parent()
}

fn same(element: Option<Ref<StyledElement>>, expected: &StyledElement) -> bool {
    element.is_some_and(|element| std::ptr::eq(&*element, expected))
}

fn create_nested_action_container(kind: NestedActionContainerKind) -> Ref<StyledElementAction> {
    match kind {
        NestedActionContainerKind::AsyncActionGroup => AsyncActionGroup::new().upcast(),
        NestedActionContainerKind::DebounceAction => DebounceAction::new().upcast(),
        NestedActionContainerKind::ThrottleAction => ThrottleAction::new().upcast(),
        NestedActionContainerKind::ConditionalActionTrueBranch
        | NestedActionContainerKind::ConditionalActionFalseBranch => ConditionalAction::new().upcast(),
        NestedActionContainerKind::SwitchCaseDefaultActions | NestedActionContainerKind::SwitchCaseCaseActions => {
            SwitchCaseAction::new().upcast()
        }
    }
}

fn add_child_action(container: &Ref<StyledElementAction>, kind: NestedActionContainerKind, child: &Ref<InvokeCommandAction>) {
    let child = child.clone().upcast();
    if let Some(group) = container.clone().cast::<AsyncActionGroup>() {
        group.actions().expect("the actions").add(child);
    } else if let Some(debounce_action) = container.clone().cast::<DebounceAction>() {
        debounce_action.actions().expect("the actions").add(child);
    } else if let Some(throttle_action) = container.clone().cast::<ThrottleAction>() {
        throttle_action.actions().expect("the actions").add(child);
    } else if let Some(conditional_action) = container.clone().cast::<ConditionalAction>() {
        if kind == NestedActionContainerKind::ConditionalActionTrueBranch {
            conditional_action.actions().expect("the actions").add(child);
        } else {
            conditional_action.else_actions().expect("the actions").add(child);
        }
    } else if let Some(switch_case_action) = container.clone().cast::<SwitchCaseAction>() {
        if kind == NestedActionContainerKind::SwitchCaseDefaultActions {
            switch_case_action.default_actions().expect("the actions").add(child);
        } else {
            let case_item = Case::new();
            case_item.set_case_value(Some(text("case")));
            case_item.actions().expect("the actions").add(child);
            switch_case_action.cases().expect("the cases").add(case_item);
        }
    } else {
        panic!("Unsupported nested action container. (Parameter 'container')");
    }
}

fn resolve_expected_child_parent(container: &Ref<StyledElementAction>, kind: NestedActionContainerKind) -> Ref<StyledElement> {
    if let Some(switch_case_action) = container.clone().cast::<SwitchCaseAction>() {
        if kind == NestedActionContainerKind::SwitchCaseCaseActions {
            return switch_case_action.cases().expect("the cases").get(0).upcast();
        }
    }

    container.clone().upcast()
}

#[test]
fn trigger_action_detaches_and_reattaches_across_associated_control_logical_tree_lifetime() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let window = create_window();
    let target = create_target();
    let trigger = ClickEventTrigger::new();
    let action = create_command_action(&command);

    trigger.actions().expect("the actions").add(action.clone().upcast());
    Interaction::get_behaviors(&target).add(trigger.clone().upcast());

    window.set_content(Some(Control::boxed(target.clone())));
    window.show();

    assert_eq!(1, command.subscription_count());
    assert!(same(parent_of(&action), &trigger));
    assert!(same(parent_of(&trigger), &target));

    window.set_content(None);
    run_jobs();

    assert_eq!(0, command.subscription_count());
    assert!(parent_of(&action).is_none());
    assert!(parent_of(&trigger).is_none());

    window.set_content(Some(Control::boxed(target.clone())));
    run_jobs();

    assert_eq!(1, command.subscription_count());
    assert!(same(parent_of(&action), &trigger));
    assert!(same(parent_of(&trigger), &target));

    window.close();
}

#[test]
fn trigger_action_detaches_when_window_closes() {
    let _app = start_app();
    let command = ObservableCommand::new();
    let window = create_window();
    let target = create_target();
    let trigger = ClickEventTrigger::new();
    let action = create_command_action(&command);

    trigger.actions().expect("the actions").add(action.clone().upcast());
    Interaction::get_behaviors(&target).add(trigger.clone().upcast());

    window.set_content(Some(Control::boxed(target.clone())));
    window.show();

    assert_eq!(1, command.subscription_count());

    window.close();
    run_jobs();

    assert_eq!(0, command.subscription_count());
    assert!(parent_of(&action).is_none());
    assert!(parent_of(&trigger).is_none());
}

#[test]
fn behavior_collection_remove_detaches_trigger_action_logical_tree() {
    let _app = start_app();
    let command = ObservableCommand::new();
    let window = create_window();
    let target = create_target();
    let trigger = ClickEventTrigger::new();
    let action = create_command_action(&command);

    trigger.actions().expect("the actions").add(action.clone().upcast());
    Interaction::get_behaviors(&target).add(trigger.clone().upcast());

    window.set_content(Some(Control::boxed(target.clone())));
    window.show();

    assert_eq!(1, command.subscription_count());

    Interaction::get_behaviors(&target).remove(&trigger.clone().upcast());

    assert_eq!(0, command.subscription_count());
    assert!(parent_of(&action).is_none());
    assert!(parent_of(&trigger).is_none());

    window.close();
}

#[test]
fn trigger_actions_replacement_detaches_old_action_logical_tree_and_attaches_new_action() {
    let _app = start_app();
    let first_command = ObservableCommand::with_result(false);
    let second_command = ObservableCommand::with_result(true);
    let window = create_window();
    let target = create_target();
    let trigger = ClickEventTrigger::new();
    let first_action = create_command_action(&first_command);
    let second_action = create_command_action(&second_command);

    trigger.actions().expect("the actions").add(first_action.clone().upcast());
    Interaction::get_behaviors(&target).add(trigger.clone().upcast());

    window.set_content(Some(Control::boxed(target.clone())));
    window.show();

    assert_eq!(1, first_command.subscription_count());
    assert!(same(parent_of(&first_action), &trigger));

    let actions = ActionCollection::new();
    actions.add(second_action.clone().upcast());
    trigger.set_actions(Some(actions));

    assert_eq!(0, first_command.subscription_count());
    assert!(parent_of(&first_action).is_none());
    assert_eq!(1, second_command.subscription_count());
    assert!(same(parent_of(&second_action), &trigger));

    window.close();
}

fn nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed(kind: NestedActionContainerKind) {
    let _app = start_app();
    let command = ObservableCommand::new();
    let window = create_window();
    let target = create_target();
    let trigger = ClickEventTrigger::new();
    let container = create_nested_action_container(kind);
    let child_action = create_command_action(&command);

    add_child_action(&container, kind, &child_action);
    trigger.actions().expect("the actions").add(container.clone().upcast());
    Interaction::get_behaviors(&target).add(trigger.clone().upcast());

    window.set_content(Some(Control::boxed(target.clone())));
    window.show();

    assert_eq!(1, command.subscription_count());
    assert!(same(parent_of(&child_action), &resolve_expected_child_parent(&container, kind)));

    Interaction::get_behaviors(&target).remove(&trigger.clone().upcast());

    assert_eq!(0, command.subscription_count());
    assert!(parent_of(&container).is_none());
    assert!(parent_of(&child_action).is_none());

    window.close();
}

#[test]
fn nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed_async_action_group() {
    nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed(
        NestedActionContainerKind::AsyncActionGroup,
    );
}

#[test]
fn nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed_debounce_action() {
    nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed(
        NestedActionContainerKind::DebounceAction,
    );
}

#[test]
fn nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed_throttle_action() {
    nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed(
        NestedActionContainerKind::ThrottleAction,
    );
}

#[test]
fn nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed_conditional_action_true_branch() {
    nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed(
        NestedActionContainerKind::ConditionalActionTrueBranch,
    );
}

#[test]
fn nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed_conditional_action_false_branch() {
    nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed(
        NestedActionContainerKind::ConditionalActionFalseBranch,
    );
}

#[test]
fn nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed_switch_case_default_actions() {
    nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed(
        NestedActionContainerKind::SwitchCaseDefaultActions,
    );
}

#[test]
fn nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed_switch_case_case_actions() {
    nested_action_containers_detach_child_action_logical_tree_when_trigger_is_removed(
        NestedActionContainerKind::SwitchCaseCaseActions,
    );
}

#[test]
fn nested_action_container_actions_replacement_detaches_old_child_action_and_attaches_new_child_action() {
    let _app = start_app();
    let first_command = ObservableCommand::with_result(false);
    let second_command = ObservableCommand::with_result(true);
    let window = create_window();
    let target = create_target();
    let trigger = ClickEventTrigger::new();
    let group = AsyncActionGroup::new();
    let first_action = create_command_action(&first_command);
    let second_action = create_command_action(&second_command);

    group.actions().expect("the actions").add(first_action.clone().upcast());
    trigger.actions().expect("the actions").add(group.clone().upcast());
    Interaction::get_behaviors(&target).add(trigger.clone().upcast());

    window.set_content(Some(Control::boxed(target.clone())));
    window.show();

    assert_eq!(1, first_command.subscription_count());
    assert!(same(parent_of(&first_action), &group));

    let actions = ActionCollection::new();
    actions.add(second_action.clone().upcast());
    group.set_actions(Some(actions));

    assert_eq!(0, first_command.subscription_count());
    assert!(parent_of(&first_action).is_none());
    assert_eq!(1, second_command.subscription_count());
    assert!(same(parent_of(&second_action), &group));

    window.close();
}

#[test]
fn nested_action_bindings_clear_when_container_logical_tree_detaches() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let window = create_window();
    let target = create_target();
    let indicator = Border::new();
    let trigger = ClickEventTrigger::new();
    let group = AsyncActionGroup::new();
    let action = create_command_action(&command);

    let _binding = indicator.bind_typed(
        InputElement::is_enabled_property(),
        action.get_observable(InvokeCommandActionBase::can_execute_command_property()),
        ferroui_base::data::BindingPriority::LocalValue,
    );
    group.actions().expect("the actions").add(action.clone().upcast());
    trigger.actions().expect("the actions").add(group.clone().upcast());
    Interaction::get_behaviors(&target).add(trigger.clone().upcast());

    let panel = StackPanel::new();
    panel.children().add(target.clone().upcast::<Control>());
    panel.children().add(indicator.clone().upcast::<Control>());
    window.set_content(Some(Control::boxed(panel)));
    window.show();

    assert_eq!(1, command.subscription_count());
    assert!(!indicator.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(indicator.is_enabled());

    Interaction::get_behaviors(&target).remove(&trigger.clone().upcast());

    assert_eq!(0, command.subscription_count());

    command.can_execute_result.set(false);
    command.raise_can_execute_changed();

    assert!(action.can_execute_command());
    assert!(indicator.is_enabled());

    window.close();
}

#[test]
fn use_command_can_execute_for_is_enabled_binding_clears_when_trigger_logical_tree_detaches() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let window = create_window();
    let target = create_target();
    let trigger = ClickEventTrigger::new();
    let action = create_command_action(&command);
    action.set_use_command_can_execute_for_is_enabled(true);

    trigger.actions().expect("the actions").add(action.clone().upcast());
    Interaction::get_behaviors(&target).add(trigger.clone().upcast());

    window.set_content(Some(Control::boxed(target.clone())));
    window.show();

    assert_eq!(1, command.subscription_count());
    assert!(!target.is_enabled());

    Interaction::get_behaviors(&target).remove(&trigger.clone().upcast());

    assert_eq!(0, command.subscription_count());
    assert!(target.is_enabled());

    command.can_execute_result.set(false);
    command.raise_can_execute_changed();

    assert!(target.is_enabled());

    window.close();
}

#[test]
fn popup_hosted_trigger_action_detaches_when_popup_closes() {
    let _app = start_app();
    let command = ObservableCommand::new();
    let window = create_window();
    let placement_target = Button::new();
    placement_target.set_content(Some(text("Host")));
    placement_target.set_width(120.0);
    placement_target.set_height(32.0);
    let popup_target = create_target();
    let popup = Popup::new();
    popup.set_placement_target(Some(placement_target.clone().upcast()));
    popup.set_child(Some(popup_target.clone().upcast()));
    let trigger = ClickEventTrigger::new();
    let action = create_command_action(&command);

    trigger.actions().expect("the actions").add(action.clone().upcast());
    Interaction::get_behaviors(&popup_target).add(trigger.clone().upcast());

    let grid = Grid::new();
    grid.children().add(placement_target.clone().upcast::<Control>());
    grid.children().add(popup.clone().upcast::<Control>());
    window.set_content(Some(Control::boxed(grid)));
    window.show();

    popup.set_is_open(true);
    run_jobs();

    assert_eq!(1, command.subscription_count());
    assert!(same(parent_of(&action), &trigger));

    popup.set_is_open(false);
    run_jobs();

    assert_eq!(0, command.subscription_count());
    assert!(parent_of(&action).is_none());
    assert!(parent_of(&trigger).is_none());

    window.close();
}

#[test]
fn top_level_hosted_trigger_detaches_action_logical_tree_when_behavior_is_removed() {
    let _app = start_app();
    let command = ObservableCommand::new();
    let window = create_window();
    let trigger = ClickEventTrigger::new();
    let action = create_command_action(&command);

    trigger.actions().expect("the actions").add(action.clone().upcast());
    Interaction::get_behaviors(&window).add(trigger.clone().upcast());

    assert_eq!(1, command.subscription_count());
    assert!(same(parent_of(&trigger), &window));
    assert!(same(parent_of(&action), &window));

    Interaction::get_behaviors(&window).remove(&trigger.clone().upcast());

    assert_eq!(0, command.subscription_count());
    assert!(parent_of(&trigger).is_none());
    assert!(parent_of(&action).is_none());
}
