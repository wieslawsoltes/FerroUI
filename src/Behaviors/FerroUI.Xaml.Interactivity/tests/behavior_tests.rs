//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/BehaviorTests.cs` of Xaml.Behaviors.

use super::support::{object, panics, start_app};
use super::test_behaviors::TestBehavior;
use crate::IBehaviorEventsHandler;
use ferroui_controls::Button;

#[test]
fn attach_sets_associated_object() {
    let _app = start_app();
    let behavior = TestBehavior::new();
    let button = Button::new();

    behavior.attach(&button);

    assert_eq!(Some(object(&button)), behavior.associated_object());
    assert_eq!(1, behavior.attached_called());
}

#[test]
fn attach_same_object_no_op() {
    let _app = start_app();
    let behavior = TestBehavior::new();
    let button = Button::new();

    behavior.attach(&button);
    behavior.attach(&button);

    assert_eq!(1, behavior.attached_called());
}

#[test]
fn attach_null_does_nothing() {
    let _app = start_app();
    let behavior = TestBehavior::new();

    behavior.attach(None);

    assert!(behavior.associated_object().is_none());
    assert_eq!(0, behavior.attached_called());
}

#[test]
fn attach_different_object_throws() {
    let _app = start_app();
    let behavior = TestBehavior::new();
    let first = Button::new();
    behavior.attach(&first);

    let second = Button::new();
    assert!(panics(|| behavior.attach(&second)));
}

#[test]
fn detach_clears_associated_object() {
    let _app = start_app();
    let behavior = TestBehavior::new();
    let button = Button::new();

    behavior.attach(&button);
    behavior.detach();

    assert!(behavior.associated_object().is_none());
    assert_eq!(1, behavior.detaching_called());
}

#[test]
fn behavior_event_methods_invoke_overrides() {
    let _app = start_app();
    let behavior = TestBehavior::new();
    let handler: &dyn IBehaviorEventsHandler = &**behavior;

    handler.attached_to_visual_tree_event_handler();
    handler.detached_from_visual_tree_event_handler();
    handler.attached_to_logical_tree_event_handler();
    handler.detached_from_logical_tree_event_handler();
    handler.loaded_event_handler();
    handler.unloaded_event_handler();
    handler.initialized_event_handler();
    handler.data_context_changed_event_handler();
    handler.resources_changed_event_handler();
    handler.actual_theme_variant_changed_event_handler();

    assert_eq!(1, behavior.visual_attach_called());
    assert_eq!(1, behavior.visual_detach_called());
    assert_eq!(1, behavior.logical_attach_called());
    assert_eq!(1, behavior.logical_detach_called());
    assert_eq!(1, behavior.loaded_called());
    assert_eq!(1, behavior.unloaded_called());
    assert_eq!(1, behavior.initialized_called());
    assert_eq!(1, behavior.data_context_changed_called());
    assert_eq!(1, behavior.resources_changed_called());
    assert_eq!(1, behavior.theme_changed_called());
}
