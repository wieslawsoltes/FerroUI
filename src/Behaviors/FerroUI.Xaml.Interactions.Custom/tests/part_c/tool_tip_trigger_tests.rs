//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/ToolTipTriggerTests.cs` of
//! Xaml.Behaviors.
//!
//! Adapted: `Assert.Same(eventArgs, commandParameter)` compares the shared handle of the args the
//! actions receive with the raised args.

use super::support::{add_behavior, start_app, window_with, Command};
use crate::{ToolTipClosingTrigger, ToolTipOpeningTrigger};
use ferroui_base::interactivity::{CancelRoutedEventArgs, IRoutedEventArgs, RoutedEventArgs, RoutingStrategies};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Border, ToolTip};
use ferroui_xaml_interactions::{EventTriggerBehavior, InvokeCommandAction};
use std::cell::RefCell;
use std::rc::Rc;

/// The parameter a command received.
type Recorded = Rc<RefCell<Option<BoxedValue>>>;

/// `new InvokeCommandAction { Command = new Command(parameter => commandParameter = parameter), PassEventArgsToCommand = true }`.
fn recording_action() -> (Ref<InvokeCommandAction>, Recorded) {
    let command_parameter: Recorded = Rc::new(RefCell::new(None));
    let recorded = command_parameter.clone();
    let action = InvokeCommandAction::new();
    action.set_command(Some(Command::new(move |parameter| *recorded.borrow_mut() = parameter.cloned())));
    action.set_pass_event_args_to_command(true);
    (action, command_parameter)
}

/// `Assert.Same(eventArgs, commandParameter)`.
fn assert_same_args(event_args: &dyn IRoutedEventArgs, command_parameter: &Recorded) {
    let parameter = command_parameter.borrow().clone().expect("the command ran with the args of the event");
    let shared = parameter.downcast_ref::<Rc<dyn IRoutedEventArgs>>().expect("the parameter is a shared handle to the args");
    assert!(shared.as_routed_event_args() == event_args.as_routed_event_args(), "the parameter is the raised args");
}

#[test]
fn tool_tip_opening_trigger_executes_actions_for_direct_event() {
    let _app = start_app();
    let target = Border::new();
    let trigger = ToolTipOpeningTrigger::new();
    let (action, command_parameter) = recording_action();
    trigger.actions().expect("the actions of the trigger").add(action.upcast());
    add_behavior(&target, &trigger);
    let window = window_with(&target);
    window.show();
    let event_args = CancelRoutedEventArgs::with_event(ToolTip::tool_tip_opening_event());

    target.raise_event(&event_args);

    assert_eq!(RoutingStrategies::DIRECT, trigger.event_routing_strategy());
    assert_same_args(&event_args, &command_parameter);
}

#[test]
fn tool_tip_closing_trigger_executes_actions_for_direct_event() {
    let _app = start_app();
    let target = Border::new();
    let trigger = ToolTipClosingTrigger::new();
    let (action, command_parameter) = recording_action();
    trigger.actions().expect("the actions of the trigger").add(action.upcast());
    add_behavior(&target, &trigger);
    let window = window_with(&target);
    window.show();
    let event_args = RoutedEventArgs::with_event(ToolTip::tool_tip_closing_event());

    target.raise_event(&event_args);

    assert_eq!(RoutingStrategies::DIRECT, trigger.event_routing_strategy());
    assert_same_args(&event_args, &command_parameter);
}

#[test]
fn event_trigger_behavior_executes_actions_for_tool_tip_opening_event() {
    let _app = start_app();
    let target = Border::new();
    let trigger = EventTriggerBehavior::new();
    trigger.set_event_name(Some("ToolTipOpening".to_string()));
    let (action, command_parameter) = recording_action();
    trigger.actions().expect("the actions of the trigger").add(action.upcast());
    add_behavior(&target, &trigger);
    let event_args = CancelRoutedEventArgs::with_event(ToolTip::tool_tip_opening_event());

    target.raise_event(&event_args);

    assert_same_args(&event_args, &command_parameter);
}

#[test]
fn event_trigger_behavior_executes_actions_for_tool_tip_closing_event() {
    let _app = start_app();
    let target = Border::new();
    let trigger = EventTriggerBehavior::new();
    trigger.set_event_name(Some("ToolTipClosing".to_string()));
    let (action, command_parameter) = recording_action();
    trigger.actions().expect("the actions of the trigger").add(action.upcast());
    add_behavior(&target, &trigger);
    let event_args = RoutedEventArgs::with_event(ToolTip::tool_tip_closing_event());

    target.raise_event(&event_args);

    assert_same_args(&event_args, &command_parameter);
}
