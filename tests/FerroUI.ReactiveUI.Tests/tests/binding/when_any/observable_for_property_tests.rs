//! `ObservableForPropertyTests` (`ReactiveUI.Binding.Tests/WhenAny/ObservableForPropertyTests.cs`).
//!
//! Upstream calls `ObservableForProperty` with an expression (the run-time form) or with a property
//! name read by reflection. The port observes a property path with `observable_for_property` and
//! its short forms, and a property name with `observable_for_property_named`, which takes the getter.
//! Upstream's overloads map as follows:
//! - `ObservableForProperty(x => ..)`: `observable_for_property_skip_initial(path, true)`;
//! - `ObservableForProperty(x => .., skipInitial)`: `observable_for_property_skip_initial(path, skipInitial)`;
//! - `ObservableForProperty(x => .., beforeChange, skipInitial, isDistinct)`: `observable_for_property`;
//! - `ObservableForProperty<TSender, TValue>(name)`: `observable_for_property_named(name, getter, false, true, true)`,
//!   and `(name, skipInitial)` is `(name, getter, false, skipInitial, true)`.
//!
//! Upstream's `EnsureInitialized` builds the application with the core services; the port observes
//! with the default observers while none is registered, so the tests need no setup.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::{HostTestFixture, NonReactiveINotifyPropertyChangedObject, ObjChain1, ObjChain2, ObjChain3, TestFixture};
use ferroui_reactiveui::*;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::prop;
use std::rc::Rc;

/// The age assigned to the sample view model.
const SAMPLE_AGE: i32 = 42;

/// The name of the property observed by the string-based overload tests.
const OBSERVED_PROPERTY_NAME: &str = "IsNotNullString";

/// The initial property value used across tests.
const INITIAL_VALUE: &str = "Initial";

/// The expected number of emitted changes when two notifications are produced.
const EXPECTED_TWO_CHANGES: usize = 2;

/// The replacement leaf value used in deep-chain tests.
const REPLACEMENT_LEAF_VALUE: i32 = 99;

/// A child fixture whose `IsOnlyOneWord` is `word`.
fn child_with_word(word: &str) -> Rc<TestFixture> {
    let child = TestFixture::new();
    child.set_is_only_one_word(word.to_string());
    child
}

/// `x => x.Child!.IsOnlyOneWord`; the value is `None` while there is no child.
fn child_word_path() -> PropertyPath<Rc<HostTestFixture>, Option<String>> {
    prop!(HostTestFixture => child).then_optional_property("IsOnlyOneWord", |child: &Rc<TestFixture>| child.is_only_one_word())
}

/// The getter of `IsNotNullString`, for the string-based overloads.
fn is_not_null_string(fixture: &Rc<TestFixture>) -> String {
    fixture.is_not_null_string()
}

#[test]
fn single_property_emits_on_change() {
    let fixture = TestFixture::new();

    let changes = collect_items(&fixture.observable_for_property_skip_initial(prop!(TestFixture => is_not_null_string), true));

    fixture.set_is_not_null_string("Bar".to_string());
    fixture.set_is_not_null_string("Baz".to_string());

    assert_eq!(changes.borrow().len(), EXPECTED_TWO_CHANGES);
    assert_eq!(changes.borrow()[0].value, "Bar");
    assert_eq!(changes.borrow()[1].value, "Baz");
}

#[test]
fn single_property_skips_initial_by_default() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string(INITIAL_VALUE.to_string());

    let changes = collect_items(&fixture.observable_for_property_skip_initial(prop!(TestFixture => is_not_null_string), true));

    // Should not have emitted the initial value.
    assert_eq!(changes.borrow().len(), 0);
}

#[test]
fn single_property_emits_initial_when_requested() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string(INITIAL_VALUE.to_string());

    let changes = collect_items(&fixture.observable_for_property_skip_initial(prop!(TestFixture => is_not_null_string), false));

    assert!(changes.borrow().len() >= 1);
    assert_eq!(changes.borrow()[0].value, INITIAL_VALUE);
}

#[test]
fn deep_chain_emits_on_leaf_change() {
    let fixture = HostTestFixture::new();
    fixture.set_child(Some(child_with_word("Foo")));

    let changes = collect_items(&fixture.observable_for_property_skip_initial(child_word_path(), false));

    assert!(changes.borrow().len() >= 1);

    fixture.child().expect("the child is set").set_is_only_one_word("Bar".to_string());

    assert!(changes.borrow().len() >= EXPECTED_TWO_CHANGES);
}

#[test]
fn deep_chain_resubscribes_on_intermediate_change() {
    let fixture = HostTestFixture::new();
    fixture.set_child(Some(child_with_word("Foo")));

    let changes = collect_items(&fixture.observable_for_property_skip_initial(child_word_path(), false));

    let initial_count = changes.borrow().len();

    // Replace the intermediate object entirely.
    fixture.set_child(Some(child_with_word("Baz")));

    assert!(changes.borrow().len() > initial_count);
}

#[test]
fn deep_chain_resubscribes_after_null_then_restore() {
    let fixture = HostTestFixture::new();
    fixture.set_child(Some(child_with_word("Foo")));

    let changes = collect_items(&fixture.observable_for_property_skip_initial(child_word_path(), false));

    let count_after_initial = changes.borrow().len();

    // Set the intermediate to null (the port produces `None` here, where upstream produces nothing).
    fixture.set_child(None);

    // Restore with a new value.
    fixture.set_child(Some(child_with_word("Bar")));

    assert!(changes.borrow().len() > count_after_initial);
}

#[test]
fn four_level_deep_chain_any_level_triggers_update() {
    let host = HostTestFixture::new();
    host.set_some_other_param(SAMPLE_AGE);
    let chain3 = ObjChain3::new();
    chain3.set_host(Some(host.clone()));
    let chain2 = ObjChain2::new();
    chain2.set_chain3(Some(chain3));
    let fixture = ObjChain1::new();
    fixture.set_chain2(Some(chain2));

    // `x => x.Chain2!.Chain3!.Host!.SomeOtherParam`: the value is `None` while a link is absent.
    let path = prop!(ObjChain1 => chain2)
        .then_optional_property_and_then("Chain3", |chain2: &Rc<ObjChain2>| chain2.chain3())
        .then_optional_property_and_then("Host", |chain3: &Rc<ObjChain3>| chain3.host())
        .then_optional_property("SomeOtherParam", |host: &Rc<HostTestFixture>| host.some_other_param());
    let values = collect_items(&fixture.observable_for_property_skip_initial(path, false));

    assert!(values.borrow().len() >= 1);

    // Change the leaf value.
    host.set_some_other_param(REPLACEMENT_LEAF_VALUE);

    assert!(values.borrow().len() >= EXPECTED_TWO_CHANGES);
    assert_eq!(values.borrow().last().map(|change| change.value), Some(Some(REPLACEMENT_LEAF_VALUE)));
}

#[test]
fn works_with_inpc_objects() {
    let obj = NonReactiveINotifyPropertyChangedObject::new();

    let changes = collect_items(&obj.observable_for_property_skip_initial(prop!(NonReactiveINotifyPropertyChangedObject => inpc_property), true));

    obj.set_inpc_property("Hello".to_string());

    assert_eq!(changes.borrow().len(), 1);
    assert_eq!(changes.borrow()[0].value, "Hello");
}

#[test]
fn string_overload_observes_property() {
    let fixture = TestFixture::new();

    let changes = collect_items(&fixture.observable_for_property_named(OBSERVED_PROPERTY_NAME, is_not_null_string, false, true, true));

    fixture.set_is_not_null_string("Test".to_string());

    assert_eq!(changes.borrow().len(), 1);
    assert_eq!(changes.borrow()[0].value, "Test");
}

#[test]
fn is_distinct_deduplicates_same_values() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string("A".to_string());

    let changes = collect_items(&fixture.observable_for_property_skip_initial(prop!(TestFixture => is_not_null_string), true));

    fixture.set_is_not_null_string("B".to_string());
    fixture.set_is_not_null_string("B".to_string()); // Same value: deduplicated by the equality check of the setter.

    assert_eq!(changes.borrow().len(), 1);
}

#[test]
fn string_overload_skip_initial_false_emits_initial_value() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string(INITIAL_VALUE.to_string());

    let changes = collect_items(&fixture.observable_for_property_named(OBSERVED_PROPERTY_NAME, is_not_null_string, false, false, true));

    assert!(changes.borrow().len() >= 1);
    assert_eq!(changes.borrow()[0].value, INITIAL_VALUE);
}

#[test]
fn string_overload_is_distinct_false_emits_all() {
    let fixture = TestFixture::new();

    let changes = collect_items(&fixture.observable_for_property_named(OBSERVED_PROPERTY_NAME, is_not_null_string, false, false, false));

    fixture.set_is_not_null_string("A".to_string());
    fixture.set_is_not_null_string("B".to_string());

    assert!(changes.borrow().len() >= EXPECTED_TWO_CHANGES);
}

#[test]
fn string_overload_before_change_emits_before_property_changes() {
    let fixture = TestFixture::new();

    // Upstream's `ObservableForProperty<TestFixture, string>(name, true)` is the overload whose
    // second argument is `skipInitial`.
    let changes = collect_items(&fixture.observable_for_property_named(OBSERVED_PROPERTY_NAME, is_not_null_string, false, true, true));

    fixture.set_is_not_null_string("Changed".to_string());

    assert!(changes.borrow().len() >= 1);
}

#[test]
fn expression_overload_is_distinct_false_emits_all() {
    let fixture = TestFixture::new();

    let changes = collect_items(&fixture.observable_for_property(prop!(TestFixture => is_not_null_string), false, false, false));

    fixture.set_is_not_null_string("A".to_string());
    fixture.set_is_not_null_string("B".to_string());

    assert!(changes.borrow().len() >= EXPECTED_TWO_CHANGES);
}

#[test]
fn expression_overload_before_change_emits_before_property_changes() {
    let fixture = TestFixture::new();

    // Upstream's `ObservableForProperty(x => .., true)` is the overload whose second argument is
    // `skipInitial`.
    let changes = collect_items(&fixture.observable_for_property_skip_initial(prop!(TestFixture => is_not_null_string), true));

    fixture.set_is_not_null_string("Changed".to_string());

    assert!(changes.borrow().len() >= 1);
}

// Not ported: StringOverload_NonExistentProperty_DoesNotThrow - reflection (the string form takes the
// getter of the property, so there is no property name that does not exist).
// Not ported: StringOverload_PropertyNotFoundViaReflection_ReturnsDefault - reflection (the value is
// read with the getter, never looked up by name).

#[test]
fn subscribe_to_expression_chain_null_value_passes_through_as_default() {
    let fixture = HostTestFixture::new();
    fixture.set_child(None);

    let values = collect_items(&fixture.observable_for_property(prop!(HostTestFixture => child), false, false, false));

    assert!(values.borrow().len() >= 1);

    // The first value is `None`, since there is no child.
    assert!(values.borrow()[0].value.is_none());
}
