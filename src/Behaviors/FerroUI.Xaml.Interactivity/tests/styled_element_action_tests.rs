//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/StyledElementActionTests.cs` of
//! Xaml.Behaviors.

use super::stub_action::StubAction;
use super::support::{boxed, object, start_app, text};
use crate::helpers::TemplatedParentHelper;
use ferroui_base::FerroObject;
use ferroui_controls::{Button, ContentControl};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn is_enabled_defaults_true() {
    let _app = start_app();
    let action = StubAction::new();

    assert!(action.is_enabled());
}

#[test]
fn is_enabled_can_be_set() {
    let _app = start_app();
    let action = StubAction::new();

    action.set_is_enabled(false);

    assert!(!action.is_enabled());
}

#[test]
fn execute_records_parameters_and_returns_value() {
    let _app = start_app();
    let action = StubAction::with_return_value(Some(text("Result")));
    let sender = boxed(&Button::new());
    let parameter = boxed(&FerroObject::new());

    let result = action.execute(Some(&sender), Some(&parameter));

    assert_eq!(Some(text("Result")), result);
    assert_eq!(1, action.execute_count());
    assert_eq!(Some(sender), action.sender());
    assert_eq!(Some(parameter), action.parameter());
}

#[test]
fn initialize_sets_is_initialized() {
    let _app = start_app();
    let action = StubAction::new();
    let initialized_count = Rc::new(Cell::new(0));
    let count = initialized_count.clone();
    let _subscription = action.initialized(move || count.set(count.get() + 1));

    assert!(!action.is_initialized());

    action.initialize();

    assert!(action.is_initialized());
    assert_eq!(1, initialized_count.get());

    // Subsequent calls should not raise event again
    action.initialize();
    assert_eq!(1, initialized_count.get());
}

#[test]
fn attach_action_to_logical_tree_sets_parent_and_templated_parent() {
    let _app = start_app();
    let action = StubAction::new();
    let parent = Button::new();
    let templated_parent = ContentControl::new();
    TemplatedParentHelper::set_templated_parent(&parent, Some(object(&templated_parent)));

    action.attach_action_to_logical_tree(&parent);

    assert_eq!(Some(parent.clone().upcast()), action.parent());
    assert_eq!(Some(object(&templated_parent)), action.templated_parent());
}

#[test]
fn detach_action_from_logical_tree_clears_parent_and_templated_parent() {
    let _app = start_app();
    let action = StubAction::new();
    let parent = Button::new();
    let templated_parent = ContentControl::new();
    TemplatedParentHelper::set_templated_parent(&parent, Some(object(&templated_parent)));

    action.attach_action_to_logical_tree(&parent);
    action.detach_action_from_logical_tree(&parent);

    assert!(action.parent().is_none());
    assert!(action.templated_parent().is_none());
}
