//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/StyledElementBehaviorTests.cs` of
//! Xaml.Behaviors.

use super::support::{object, panics, run_jobs, start_app, window_with};
use crate::helpers::TemplatedParentHelper;
use crate::{IBehaviorEventsHandler, Interaction, StyledElementBehavior, StyledElementBehaviorImpl};
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::{Button, ContentControl};

/// `TestStyledElementBehavior`: a styled element behavior with no hooks.
#[repr(C)]
struct TestStyledElementBehavior {
    base: StyledElementBehavior,
}

ferro_class!(TestStyledElementBehavior: StyledElementBehavior);
ferroui_base::ferro_impl_classes!(TestStyledElementBehavior: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

impl TestStyledElementBehavior {
    fn new() -> Ref<Self> {
        instantiate(Self { base: StyledElementBehavior::construct() })
    }
}

/// `ThrowingDetachBehavior`: a styled element behavior whose hook of the detachment from the
/// visual tree fails.
#[repr(C)]
struct ThrowingDetachBehavior {
    base: StyledElementBehavior,
}

ferro_class!(ThrowingDetachBehavior: StyledElementBehavior);
ferroui_base::ferro_impl_classes!(ThrowingDetachBehavior: FerroObjectImpl, StyledElementImpl);

impl StyledElementBehaviorImpl for ThrowingDetachBehavior {
    fn on_detached_from_visual_tree(_this: &Self) {
        panic!("Expected test exception.");
    }
}

impl ThrowingDetachBehavior {
    fn new() -> Ref<Self> {
        instantiate(Self { base: StyledElementBehavior::construct() })
    }
}

#[test]
fn detach_clears_logical_parent_and_templated_parent() {
    let _app = start_app();
    let behavior = TestStyledElementBehavior::new();
    let button = Button::new();
    let templated_parent = ContentControl::new();
    let window = window_with(Some(button.clone().upcast()));

    TemplatedParentHelper::set_templated_parent(&button, Some(object(&templated_parent)));
    Interaction::get_behaviors(&button).add(object(&behavior));
    window.show();

    assert_eq!(Some(button.clone().upcast()), behavior.parent());
    assert_eq!(Some(object(&templated_parent)), behavior.templated_parent());

    behavior.detach();

    assert!(behavior.parent().is_none());
    assert!(behavior.templated_parent().is_none());

    window.close();
}

#[test]
fn detach_visual_tree_clears_logical_scope_when_callback_throws() {
    let _app = start_app();
    let behavior = ThrowingDetachBehavior::new();
    let button = Button::new();
    let templated_parent = ContentControl::new();
    let window = window_with(Some(button.clone().upcast()));

    TemplatedParentHelper::set_templated_parent(&button, Some(object(&templated_parent)));
    Interaction::get_behaviors(&button).add(object(&behavior));
    window.show();

    assert!(panics(|| behavior.detached_from_visual_tree_event_handler()));

    assert!(behavior.parent().is_none());
    assert!(behavior.templated_parent().is_none());

    behavior.detach();
    Interaction::set_behaviors(&button, None);
    window.close();
}

#[test]
fn top_level_close_preserves_logical_parent_until_deferred_detach() {
    let _app = start_app();
    let behavior = TestStyledElementBehavior::new();
    let window = window_with(None);
    Interaction::get_behaviors(&window).add(object(&behavior));

    window.show();

    assert_eq!(Some(object(&window)), behavior.associated_object());
    assert_eq!(Some(window.clone().upcast()), behavior.parent());

    window.close();

    assert_eq!(Some(object(&window)), behavior.associated_object());
    assert_eq!(Some(window.clone().upcast()), behavior.parent());

    run_jobs();

    assert!(behavior.associated_object().is_none());
    assert!(behavior.parent().is_none());
}
