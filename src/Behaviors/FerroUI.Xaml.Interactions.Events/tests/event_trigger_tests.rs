//! Not from the reference: the triggers of the crate run their actions when their event is raised
//! on the associated object, with the args of the event as the parameter and the associated
//! object as the sender, until the associated object leaves the visual tree.

use super::support::{object, panics, start_app, Host, RecordingAction};
use crate::*;
use ferroui_base::input::{DragDrop, InputElement};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutingStrategies};
use ferroui_base::{FerroObject, Ref};
use ferroui_xaml_interactivity::{ActionCollection, Interaction};
use std::rc::Rc;

/// Shows a host with `trigger` attached to its border, raises the event with `raise` and checks
/// that the actions of the trigger ran with the args, then removes the border from the visual
/// tree and checks that raising the event again runs nothing.
fn assert_trigger_runs_actions(
    trigger: Ref<FerroObject>,
    actions: Option<ActionCollection>,
    raise: impl Fn(&Host) -> Rc<dyn IRoutedEventArgs>,
) {
    let host = Host::new();
    let action = RecordingAction::add_to(actions);
    host.add_behavior(trigger);
    host.window.show();

    assert_eq!(0, action.execute_count());

    let args = raise(&host);

    assert!(action.execute_count() >= 1, "the actions run when the event is raised");
    let parameter = action.parameter().expect("the args of the event are the parameter of the actions");
    let shared = parameter
        .downcast_ref::<Rc<dyn IRoutedEventArgs>>()
        .expect("the parameter is a shared handle to the args");
    assert!(shared.as_routed_event_args() == args.as_routed_event_args(), "the parameter is the raised args");
    let sender = action.sender().expect("the associated object is the sender of the actions");
    assert_eq!(Some(&object(&host.border)), sender.downcast_ref::<Ref<FerroObject>>());

    host.detach_border();
    let count = action.execute_count();
    raise(&host);

    assert_eq!(count, action.execute_count(), "the handler is removed when the border leaves the visual tree");

    host.window.close();
}

#[test]
fn double_tapped_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = DoubleTappedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.tapped_event_args(InputElement::double_tapped_event()))
    });
}

#[test]
fn drag_enter_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = DragEnterEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.drag_event_args(DragDrop::drag_enter_event()))
    });
}

#[test]
fn drag_leave_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = DragLeaveEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.drag_event_args(DragDrop::drag_leave_event()))
    });
}

#[test]
fn drag_over_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = DragOverEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.drag_event_args(DragDrop::drag_over_event()))
    });
}

#[test]
fn drop_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = DropEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.drag_event_args(DragDrop::drop_event()))
    });
}

#[test]
fn got_focus_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = GotFocusEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.focus_changed_event_args(InputElement::got_focus_event()))
    });
}

#[test]
fn key_down_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = KeyDownEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.key_event_args(InputElement::key_down_event()))
    });
}

#[test]
fn key_up_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = KeyUpEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.key_event_args(InputElement::key_up_event()))
    });
}

#[test]
fn lost_focus_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = LostFocusEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.focus_changed_event_args(InputElement::lost_focus_event()))
    });
}

#[test]
fn pointer_capture_lost_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = PointerCaptureLostEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.pointer_capture_lost_event_args())
    });
}

#[test]
fn pointer_entered_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = PointerEnteredEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.pointer_event_args(InputElement::pointer_entered_event()))
    });
}

#[test]
fn pointer_exited_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = PointerExitedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.pointer_event_args(InputElement::pointer_exited_event()))
    });
}

#[test]
fn pointer_moved_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = PointerMovedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.pointer_event_args(InputElement::pointer_moved_event()))
    });
}

#[test]
fn pointer_pressed_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = PointerPressedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| host.raise(host.pointer_pressed_event_args()));
}

#[test]
fn pointer_released_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = PointerReleasedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.pointer_released_event_args())
    });
}

#[test]
fn pointer_wheel_changed_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = PointerWheelChangedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| host.raise(host.pointer_wheel_event_args()));
}

#[test]
fn right_tapped_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = RightTappedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.tapped_event_args(InputElement::right_tapped_event()))
    });
}

#[test]
fn scroll_gesture_ended_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = ScrollGestureEndedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.scroll_gesture_ended_event_args())
    });
}

#[test]
fn scroll_gesture_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = ScrollGestureEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| host.raise(host.scroll_gesture_event_args()));
}

#[test]
fn tapped_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = TappedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.tapped_event_args(InputElement::tapped_event()))
    });
}

#[test]
fn text_input_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = TextInputEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| host.raise(host.text_input_event_args()));
}

#[test]
fn text_input_method_client_requested_event_trigger_runs_actions() {
    let _app = start_app();
    let trigger = TextInputMethodClientRequestedEventTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.text_input_method_client_requested_event_args())
    });
}

#[test]
fn pointer_events_trigger_runs_actions_for_pressed_released_and_moved() {
    let _app = start_app();
    let trigger = PointerEventsTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| host.raise(host.pointer_pressed_event_args()));

    let trigger = PointerEventsTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.pointer_released_event_args())
    });

    let trigger = PointerEventsTrigger::new();
    assert_trigger_runs_actions(object(&trigger), trigger.actions(), |host| {
        host.raise(host.pointer_event_args(InputElement::pointer_moved_event()))
    });
}

#[test]
fn trigger_does_not_run_actions_for_other_events() {
    let _app = start_app();
    let host = Host::new();
    let trigger = KeyDownEventTrigger::new();
    let action = RecordingAction::add_to(trigger.actions());
    host.add_behavior(object(&trigger));
    host.window.show();

    host.raise(host.key_event_args(InputElement::key_up_event()));
    host.raise(host.tapped_event_args(InputElement::tapped_event()));

    assert_eq!(0, action.execute_count());

    host.window.close();
}

#[test]
fn disabled_trigger_does_not_run_actions() {
    let _app = start_app();
    let host = Host::new();
    let trigger = TappedEventTrigger::new();
    let action = RecordingAction::add_to(trigger.actions());
    host.add_behavior(object(&trigger));
    host.window.show();

    trigger.set_is_enabled(false);
    host.raise(host.tapped_event_args(InputElement::tapped_event()));

    assert_eq!(0, action.execute_count());

    trigger.set_is_enabled(true);
    host.raise(host.tapped_event_args(InputElement::tapped_event()));

    assert_eq!(1, action.execute_count());

    host.window.close();
}

#[test]
fn trigger_listens_for_the_event_while_attached_to_the_visual_tree() {
    let _app = start_app();
    let host = Host::new();
    let trigger = TappedEventTrigger::new();
    let action = RecordingAction::add_to(trigger.actions());
    host.window.show();
    host.detach_border();
    host.add_behavior(object(&trigger));

    // The border is not in the visual tree.
    host.raise(host.tapped_event_args(InputElement::tapped_event()));
    assert_eq!(0, action.execute_count());

    host.panel.children().add(&host.border);
    host.raise(host.tapped_event_args(InputElement::tapped_event()));
    assert_eq!(1, action.execute_count());

    host.detach_border();
    host.raise(host.tapped_event_args(InputElement::tapped_event()));
    assert_eq!(1, action.execute_count());

    host.window.close();
}

#[test]
fn routing_strategies_select_the_routes_the_trigger_listens_to() {
    let _app = start_app();

    // On the parent of the border, a bubbling event raised on the border reaches the trigger
    // that listens to the bubble route.
    let host = Host::new();
    let bubbling = TappedEventTrigger::new();
    let bubbling_action = RecordingAction::add_to(bubbling.actions());
    add_to_panel(&host, object(&bubbling));
    host.window.show();
    host.raise(host.tapped_event_args(InputElement::tapped_event()));
    assert_eq!(1, bubbling_action.execute_count());
    host.window.close();

    // A trigger that listens to the direct route only does not see it.
    let host = Host::new();
    let direct = TappedEventTrigger::new();
    direct.set_routing_strategies(RoutingStrategies::DIRECT);
    let direct_action = RecordingAction::add_to(direct.actions());
    add_to_panel(&host, object(&direct));
    host.window.show();
    host.raise(host.tapped_event_args(InputElement::tapped_event()));
    assert_eq!(0, direct_action.execute_count());
    host.window.close();
}

/// Adds `trigger` to the behaviors of the panel that holds the border.
fn add_to_panel(host: &Host, trigger: Ref<FerroObject>) {
    Interaction::get_behaviors(&host.panel).add(trigger);
}

#[test]
fn tunnel_and_bubble_triggers_see_the_event_on_both_routes() {
    let _app = start_app();
    let host = Host::new();
    let trigger = KeyDownEventTrigger::new();
    let action = RecordingAction::add_to(trigger.actions());
    host.add_behavior(object(&trigger));
    host.window.show();

    host.raise(host.key_event_args(InputElement::key_down_event()));

    // The event tunnels to the border, then bubbles from it.
    assert_eq!(2, action.execute_count());

    host.window.close();
}

#[test]
fn routing_strategies_default_to_the_routes_of_the_reference() {
    let _app = start_app();
    let bubble = RoutingStrategies::BUBBLE;
    let direct = RoutingStrategies::DIRECT;
    let tunnel_bubble = RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE;

    assert_eq!(bubble, DoubleTappedEventTrigger::new().routing_strategies());
    assert_eq!(bubble, DragEnterEventTrigger::new().routing_strategies());
    assert_eq!(bubble, DragLeaveEventTrigger::new().routing_strategies());
    assert_eq!(bubble, DragOverEventTrigger::new().routing_strategies());
    assert_eq!(bubble, DropEventTrigger::new().routing_strategies());
    assert_eq!(bubble, GotFocusEventTrigger::new().routing_strategies());
    assert_eq!(tunnel_bubble, KeyDownEventTrigger::new().routing_strategies());
    assert_eq!(tunnel_bubble, KeyUpEventTrigger::new().routing_strategies());
    assert_eq!(bubble, LostFocusEventTrigger::new().routing_strategies());
    assert_eq!(direct, PointerCaptureLostEventTrigger::new().routing_strategies());
    assert_eq!(direct, PointerEnteredEventTrigger::new().routing_strategies());
    assert_eq!(tunnel_bubble, PointerEventsTrigger::new().routing_strategies());
    assert_eq!(direct, PointerExitedEventTrigger::new().routing_strategies());
    assert_eq!(tunnel_bubble, PointerMovedEventTrigger::new().routing_strategies());
    assert_eq!(tunnel_bubble, PointerPressedEventTrigger::new().routing_strategies());
    assert_eq!(tunnel_bubble, PointerReleasedEventTrigger::new().routing_strategies());
    assert_eq!(tunnel_bubble, PointerWheelChangedEventTrigger::new().routing_strategies());
    assert_eq!(bubble, RightTappedEventTrigger::new().routing_strategies());
    assert_eq!(bubble, ScrollGestureEndedEventTrigger::new().routing_strategies());
    assert_eq!(bubble, ScrollGestureEventTrigger::new().routing_strategies());
    assert_eq!(bubble, TappedEventTrigger::new().routing_strategies());
    assert_eq!(tunnel_bubble, TextInputEventTrigger::new().routing_strategies());
    assert_eq!(tunnel_bubble, TextInputMethodClientRequestedEventTrigger::new().routing_strategies());
}

#[test]
fn trigger_attached_to_an_object_that_is_not_interactive_panics() {
    let _app = start_app();
    let trigger = TappedEventTrigger::new();
    let not_interactive = RecordingAction::new();

    // `StyledElementTrigger<Interactive>` of the reference: the associated object is checked on
    // attach.
    assert!(panics(|| trigger.attach(object(&not_interactive))));
}
