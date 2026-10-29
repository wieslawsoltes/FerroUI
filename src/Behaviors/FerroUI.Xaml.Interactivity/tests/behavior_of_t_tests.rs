//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/BehaviorOfTTests.cs` of Xaml.Behaviors.

use super::support::{object, panics, start_app};
use super::test_behaviors::TestBehaviorOfT;
use ferroui_controls::{Button, TextBlock};

#[test]
fn attach_correct_type_sets_typed_associated_object() {
    let _app = start_app();
    let behavior = TestBehaviorOfT::new();
    let button = Button::new();

    behavior.attach(&button);

    assert_eq!(Some(button.clone()), behavior.associated_object_as::<Button>());
    assert_eq!(Some(object(&button)), behavior.associated_object());
    assert_eq!(1, behavior.attached_called());
}

#[test]
fn attach_wrong_type_throws() {
    let _app = start_app();
    let behavior = TestBehaviorOfT::new();
    let text_block = TextBlock::new();

    assert!(panics(|| behavior.attach(&text_block)));
}

#[test]
fn detach_after_attach_clears_associated_object() {
    let _app = start_app();
    let behavior = TestBehaviorOfT::new();
    let button = Button::new();

    behavior.attach(&button);
    behavior.detach();

    assert!(behavior.associated_object_as::<Button>().is_none());
    assert_eq!(1, behavior.detaching_called());
}
