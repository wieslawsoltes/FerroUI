//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/Collections/BehaviorCollectionTest.cs`
//! of Xaml.Behaviors.
//!
//! Adapted: the hosts are held by locals of the tests. A collection and its behaviors refer to
//! their host weakly (the host holds them), so a host created inline (`Attach(new Button())`) would
//! be released at once.

use crate::tests::stub_behavior::StubBehavior;
use crate::tests::support::{behavior_collection, object, start_app};
use crate::tests::test_uitilties::TestUtilities;
use crate::BehaviorCollection;
use ferroui_base::{FerroObject, Ref};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Button, StackPanel, TextBlock};

fn stub(collection: &BehaviorCollection, index: usize) -> Ref<StubBehavior> {
    collection.get(index).cast::<StubBehavior>().expect("a stub behavior")
}

fn stubs(collection: &BehaviorCollection) -> Vec<Ref<StubBehavior>> {
    (0..collection.count()).map(|index| stub(collection, index)).collect()
}

#[test]
fn vector_changed_non_behavior_added_exception_thrown() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    behavior_collection.add(object(&StubBehavior::new()));

    TestUtilities::assert_throws_invalid_operation_exception(|| behavior_collection.add(object(&TextBlock::new())));
}

#[test]
fn vector_changed_behavior_changed_to_non_behavior_exception_thrown() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    behavior_collection.add(object(&StubBehavior::new()));

    TestUtilities::assert_throws_invalid_operation_exception(|| behavior_collection.set(0, object(&ToggleButton::new())));
}

#[test]
fn vector_changed_duplicate_add_exception_thrown() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    let stub = StubBehavior::new();
    behavior_collection.add(object(&stub));

    TestUtilities::assert_throws_invalid_operation_exception(|| behavior_collection.add(object(&stub)));
}

#[test]
fn vector_changed_add_while_not_attached_attach_not_called() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    let stub = StubBehavior::new();
    behavior_collection.add(object(&stub));

    TestUtilities::assert_not_attached(&stub);
}

#[test]
fn vector_changed_add_while_attached_all_attached() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    let button = Button::new();
    behavior_collection.attach(&button);

    behavior_collection.add(object(&StubBehavior::new()));
    behavior_collection.add(object(&StubBehavior::new()));
    behavior_collection.add(object(&StubBehavior::new()));

    for stub in stubs(&behavior_collection) {
        TestUtilities::assert_attached(&stub, behavior_collection.associated_object());
    }
}

#[test]
fn vector_changed_replace_while_attached_old_detached_new_attached() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    let button = Button::new();
    behavior_collection.attach(&button);

    let first = StubBehavior::new();
    behavior_collection.add(object(&first));

    let second = StubBehavior::new();

    behavior_collection.set(0, object(&second));

    TestUtilities::assert_detached(&first);

    TestUtilities::assert_attached(&second, behavior_collection.associated_object());
}

#[test]
fn vector_changed_remove_while_not_attached_detach_not_called() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();

    let behavior = StubBehavior::new();
    behavior_collection.add(object(&behavior));
    behavior_collection.remove(&object(&behavior));

    TestUtilities::assert_not_detached(&behavior);
}

#[test]
fn vector_changed_remove_while_attached_detached() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    let toggle_button = ToggleButton::new();
    behavior_collection.attach(&toggle_button);

    let behavior = StubBehavior::new();
    behavior_collection.add(object(&behavior));
    behavior_collection.remove(&object(&behavior));

    TestUtilities::assert_detached(&behavior);
}

#[test]
fn vector_changed_reset_while_not_attached_detach_not_called() {
    let _app = start_app();
    let behavior_array = [StubBehavior::new(), StubBehavior::new(), StubBehavior::new()];

    let behavior_collection = BehaviorCollection::new();
    for behavior in &behavior_array {
        behavior_collection.add(object(behavior));
    }

    behavior_collection.clear();

    for behavior in &behavior_array {
        TestUtilities::assert_not_detached(behavior);
    }
}

#[test]
fn vector_changed_reset_while_attached_all_detached() {
    let _app = start_app();
    let behavior_array = [StubBehavior::new(), StubBehavior::new(), StubBehavior::new()];

    let behavior_collection = BehaviorCollection::new();
    let button = Button::new();
    behavior_collection.attach(&button);

    for behavior in &behavior_array {
        behavior_collection.add(object(behavior));
    }

    behavior_collection.clear();

    for behavior in &behavior_array {
        TestUtilities::assert_detached(behavior);
    }
}

#[test]
fn attach_multiple_behaviors_all_attached() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    behavior_collection.add(object(&StubBehavior::new()));
    behavior_collection.add(object(&StubBehavior::new()));
    behavior_collection.add(object(&StubBehavior::new()));

    let button = Button::new();
    behavior_collection.attach(&button);

    assert_eq!(
        Some(object(&button)),
        behavior_collection.associated_object(),
        "Attach should set the AssociatedObject to the given parameter."
    );

    for stub in stubs(&behavior_collection) {
        TestUtilities::assert_attached(&stub, Some(object(&button)));
    }
}

#[test]
fn attach_null_attach_not_called_on_items() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    behavior_collection.add(object(&StubBehavior::new()));
    behavior_collection.add(object(&StubBehavior::new()));
    behavior_collection.add(object(&StubBehavior::new()));

    behavior_collection.attach(None::<Ref<FerroObject>>);

    for stub in stubs(&behavior_collection) {
        TestUtilities::assert_not_attached(&stub);
    }
}

#[test]
fn attach_multiple_objects_exception_thrown() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    let _stub = StubBehavior::new();
    let button = Button::new();
    behavior_collection.attach(&button);

    let stack_panel = StackPanel::new();
    TestUtilities::assert_throws_invalid_operation_exception(|| behavior_collection.attach(&stack_panel));
}

#[test]
fn attach_non_null_then_null_exception_thrown() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    behavior_collection.add(object(&StubBehavior::new()));

    let button = Button::new();
    behavior_collection.attach(&button);

    TestUtilities::assert_throws_invalid_operation_exception(|| behavior_collection.attach(None::<Ref<FerroObject>>));
}

#[test]
fn attach_multiple_time_same_object_attach_called_once() {
    let _app = start_app();
    let behavior_collection = behavior_collection(&[object(&StubBehavior::new())]);

    let button = Button::new();
    behavior_collection.attach(&button);
    behavior_collection.attach(&button);

    // This method hard codes AttachCount == 1.
    TestUtilities::assert_attached(&stub(&behavior_collection, 0), Some(object(&button)));
}

#[test]
fn detach_not_attached_detach_not_called_on_items() {
    let _app = start_app();
    let behavior_collection = behavior_collection(&[object(&StubBehavior::new())]);

    behavior_collection.detach();

    TestUtilities::assert_not_detached(&stub(&behavior_collection, 0));
}

#[test]
fn detach_attached_all_items_detached() {
    let _app = start_app();
    let behavior_collection = BehaviorCollection::new();
    behavior_collection.add(object(&StubBehavior::new()));
    behavior_collection.add(object(&StubBehavior::new()));
    behavior_collection.add(object(&StubBehavior::new()));

    let button = Button::new();
    behavior_collection.attach(&button);
    behavior_collection.detach();

    assert!(behavior_collection.associated_object().is_none(), "The AssociatedObject should be null after Detach.");

    for behavior in stubs(&behavior_collection) {
        TestUtilities::assert_detached(&behavior);
    }
}
