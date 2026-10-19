//! `ObservedChangedMixinTests` (`ReactiveUI.Binding.Tests/ObservableForProperty/ObservedChangedMixinTests.cs`).
//!
//! Upstream builds the expression of a change with `Reflection.Rewrite`; a change of the port
//! carries the text of a property path (`PropertyPath::path`), which is what `GetPropertyName`
//! reads, and always carries its value.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::{TestAddress, TestViewModel};
use ferroui_reactive::Observable;
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// The age assigned to the sample view model.
const SAMPLE_AGE: i32 = 42;

/// A directly populated change value used across tests.
const DIRECT_VALUE: &str = "Direct";

/// A sample integer value carried by an observed change.
const SAMPLE_INT_VALUE: i32 = 99;

/// The expected number of mapped values from a two-element change stream.
const EXPECTED_TWO_VALUES: usize = 2;

/// `x => x.Name` as the expression of a change.
fn name_expression() -> String {
    prop!(TestViewModel => name).path()
}

/// A view model whose `Name` is `name`.
fn view_model_named(name: &str) -> Rc<TestViewModel> {
    let vm = TestViewModel::new();
    vm.set_name(name.to_string());
    vm
}

#[test]
fn get_property_name_simple_property_returns_name() {
    let change = ObservedChange::new(TestViewModel::new(), name_expression(), "Hello".to_string());

    let name = change.get_property_name();

    assert_eq!(name, "Name");
}

#[test]
fn get_property_name_nested_property_returns_dotted_path() {
    let expression = prop!(TestViewModel => address).then_optional_property("City", |address: &Rc<TestAddress>| address.city()).path();
    let change = ObservedChange::new(TestViewModel::new(), expression, "Seattle".to_string());

    let name = change.get_property_name();

    assert_eq!(name, "Address.City");
}

#[test]
fn get_value_with_populated_value_returns_value() {
    let change = ObservedChange::new(view_model_named("Test"), name_expression(), DIRECT_VALUE.to_string());

    let value = change.get_value();

    assert_eq!(value, DIRECT_VALUE);
}

// Not ported: GetValue_WithDefaultValue_EvaluatesChain - expression trees (upstream reads the value
// again through the expression when the change carries the default; a change of the port always
// carries its value).
// Not ported: GetValueOrDefault_NullInChain_ReturnsDefault - expression trees (same reason).
// Not ported: GetPropertyName_NullItem_ThrowsArgumentNullException - argument validation.
// Not ported: GetValue_NullItem_ThrowsArgumentNullException - argument validation.
// Not ported: GetValueOrDefault_NullItem_ThrowsArgumentNullException - argument validation.

#[test]
fn get_value_non_default_value_returns_direct_value() {
    let vm = TestViewModel::new();
    vm.set_age(SAMPLE_AGE);
    let change = ObservedChange::new(vm, prop!(TestViewModel => age).path(), SAMPLE_INT_VALUE);

    let value = change.get_value();

    assert_eq!(value, SAMPLE_INT_VALUE);
}

#[test]
fn get_value_or_default_with_non_default_value_returns_value() {
    let change = ObservedChange::new(view_model_named("Test"), name_expression(), DIRECT_VALUE.to_string());

    let value = change.get_value_or_default();

    assert_eq!(value, DIRECT_VALUE);
}

// Not ported: GetValue_BrokenChain_ThrowsException - expression trees (the value is read again
// through the expression of the change).

#[test]
fn value_maps_changes_to_values() {
    let change1 = ObservedChange::new(TestViewModel::new(), name_expression(), "First".to_string());
    let change2 = ObservedChange::new(TestViewModel::new(), name_expression(), "Second".to_string());

    let stream: Rc<dyn IObservable<ObservedChange<Rc<TestViewModel>, String>>> = Observable::to_observable(vec![change1, change2]);
    let values = collect_items(&stream.value());

    assert_eq!(values.borrow().len(), EXPECTED_TWO_VALUES);
    assert_eq!(values.borrow()[0], "First");
    assert_eq!(values.borrow()[1], "Second");
}

// Not ported: SetValueToProperty_ValidTarget_SetsValue - reflection (`SetValueToProperty` writes
// through a property chain by reflection and is not ported).
// Not ported: SetValueToProperty_NullTarget_DoesNotThrow - reflection (same reason).
