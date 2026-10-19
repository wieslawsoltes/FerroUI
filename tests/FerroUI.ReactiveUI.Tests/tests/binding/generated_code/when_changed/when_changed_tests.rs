//! `WhenChangedTests` (`ReactiveUI.Binding.GeneratedCode.Tests/WhenChanged/WhenChangedTests.cs`).
//!
//! Upstream's `PropertyValues<T1, ..., Tn>` is a tuple: `Property1` is `.0`.

use crate::mocks::collect_items;
use crate::tests::binding::generated_code::test_models::scenarios::when_changed_scenarios;
use crate::tests::binding::generated_code::test_models::{BigViewModel, TestViewModel};
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;

/// The initial name value.
const INITIAL_NAME: &str = "Initial";

/// The minimum emission count after an initial value plus one change.
const MIN_EMISSIONS_AFTER_CHANGE: usize = 2;

/// The minimum emission count after an initial value plus two changes.
const MIN_EMISSIONS_AFTER_TWO_CHANGES: usize = 3;

/// The minimum emission count after an initial value plus three changes.
const MIN_EMISSIONS_AFTER_THREE_CHANGES: usize = 4;

/// The index of the second emission.
const SECOND_EMISSION_INDEX: usize = 1;

/// The index of the third emission.
const THIRD_EMISSION_INDEX: usize = 2;

/// The index of the fourth emission.
const FOURTH_EMISSION_INDEX: usize = 3;

/// The integer value of the two-property tests.
const TWO_PROP_INT_VALUE: i32 = 42;

/// The replacement integer value.
const REPLACEMENT_INT_VALUE: i32 = 2;

/// The integer value of the three-property test.
const THREE_PROP_INT_VALUE: i32 = 10;

/// The value of the first property.
const FIRST_PROPERTY_VALUE: &str = "Hello";

/// The initial city.
const INITIAL_CITY: &str = "Seattle";

/// The integer value of the four-property test.
const FOUR_PROP_INT_VALUE: i32 = 20;

/// The double value of the three-property test.
#[allow(clippy::approx_constant)]
const THREE_PROP_DOUBLE_VALUE: f64 = 3.14;

/// The double value of the four-property test.
const FOUR_PROP_DOUBLE_VALUE: f64 = 2.71;

/// A view model named `name`.
fn named(name: &str) -> Rc<TestViewModel> {
    let vm = TestViewModel::new();
    vm.set_name(name.to_string());
    vm
}

#[test]
fn single_property_emits_initial_value() {
    let vm = named(INITIAL_NAME);

    let values = collect_items(&when_changed_scenarios::single_property_name(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], INITIAL_NAME);
}

#[test]
fn single_property_emits_on_change() {
    let vm = named(INITIAL_NAME);

    let values = collect_items(&when_changed_scenarios::single_property_name(&vm));

    vm.set_name("Changed".to_string());

    assert!(values.borrow().len() >= MIN_EMISSIONS_AFTER_CHANGE);
    assert!(values.borrow().contains(&"Changed".to_string()));
}

#[test]
fn single_property_emits_sequential_changes() {
    let vm = named("A");

    let values = collect_items(&when_changed_scenarios::single_property_name(&vm));

    vm.set_name("B".to_string());
    vm.set_name("C".to_string());
    vm.set_name("D".to_string());

    assert!(values.borrow().len() >= MIN_EMISSIONS_AFTER_THREE_CHANGES);
    assert_eq!(values.borrow()[0], "A");
    assert_eq!(values.borrow()[SECOND_EMISSION_INDEX], "B");
    assert_eq!(values.borrow()[THIRD_EMISSION_INDEX], "C");
    assert_eq!(values.borrow()[FOURTH_EMISSION_INDEX], "D");
}

#[test]
fn two_properties_emits_initial_tuple() {
    let vm = BigViewModel::new();
    vm.set_prop1(FIRST_PROPERTY_VALUE.to_string());
    vm.set_prop2(TWO_PROP_INT_VALUE);

    let values = collect_items(&when_changed_scenarios::two_properties(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0].0, FIRST_PROPERTY_VALUE);
    assert_eq!(values.borrow()[0].1, TWO_PROP_INT_VALUE);
}

#[test]
fn two_properties_emits_on_either_change() {
    let vm = BigViewModel::new();
    vm.set_prop1("A".to_string());
    vm.set_prop2(1);

    let values = collect_items(&when_changed_scenarios::two_properties(&vm));

    vm.set_prop1("B".to_string());

    assert!(values.borrow().len() >= MIN_EMISSIONS_AFTER_CHANGE);
    assert_eq!(values.borrow().last(), Some(&("B".to_string(), 1)));

    vm.set_prop2(REPLACEMENT_INT_VALUE);

    assert!(values.borrow().len() >= MIN_EMISSIONS_AFTER_TWO_CHANGES);
    assert_eq!(values.borrow().last(), Some(&("B".to_string(), REPLACEMENT_INT_VALUE)));
}

#[test]
fn three_properties_emits_initial_values() {
    let vm = BigViewModel::new();
    vm.set_prop1("X".to_string());
    vm.set_prop2(THREE_PROP_INT_VALUE);
    vm.set_prop3(THREE_PROP_DOUBLE_VALUE);

    let values = collect_items(&when_changed_scenarios::three_properties(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0].0, "X");
    assert_eq!(values.borrow()[0].1, THREE_PROP_INT_VALUE);
    assert_eq!(values.borrow()[0].2, THREE_PROP_DOUBLE_VALUE);
}

#[test]
fn four_properties_emits_initial_values() {
    let vm = BigViewModel::new();
    vm.set_prop1("Y".to_string());
    vm.set_prop2(FOUR_PROP_INT_VALUE);
    vm.set_prop3(FOUR_PROP_DOUBLE_VALUE);
    vm.set_prop4(true);

    let values = collect_items(&when_changed_scenarios::four_properties(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0].0, "Y");
    assert_eq!(values.borrow()[0].1, FOUR_PROP_INT_VALUE);
    assert_eq!(values.borrow()[0].2, FOUR_PROP_DOUBLE_VALUE);
    assert!(values.borrow()[0].3);
}

#[test]
fn with_selector_combines_values() {
    let vm = BigViewModel::new();
    vm.set_prop1(FIRST_PROPERTY_VALUE.to_string());
    vm.set_prop2(TWO_PROP_INT_VALUE);

    let values = collect_items(&when_changed_scenarios::with_selector_two_properties(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], "Hello_42");
}

#[test]
fn deep_chain_emits_nested_property_value() {
    let vm = BigViewModel::new();
    vm.address().set_city(INITIAL_CITY.to_string());

    let values = collect_items(&when_changed_scenarios::deep_chain_address_city(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], INITIAL_CITY);
}

#[test]
fn deep_chain_emits_on_nested_property_change() {
    let vm = BigViewModel::new();
    vm.address().set_city(INITIAL_CITY.to_string());

    let values = collect_items(&when_changed_scenarios::deep_chain_address_city(&vm));

    vm.address().set_city("Portland".to_string());

    assert!(values.borrow().len() >= MIN_EMISSIONS_AFTER_CHANGE);
    assert!(values.borrow().contains(&"Portland".to_string()));
}

#[test]
fn disposal_stops_listening() {
    let vm = named(INITIAL_NAME);
    let values = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = values.clone();
    let subscription = when_changed_scenarios::single_property_name(&vm).subscribe_next(move |value| sink.borrow_mut().push(value));

    subscription.dispose();

    vm.set_name("AfterDisposal".to_string());

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], INITIAL_NAME);
}
