//! `WhenChangingTests` (`ReactiveUI.Binding.GeneratedCode.Tests/WhenChanged/WhenChangingTests.DeepChain.cs`).

use crate::mocks::collect_items;
use crate::tests::binding::generated_code::test_models::scenarios::when_changing_scenarios;
use crate::tests::binding::generated_code::test_models::BigViewModel;
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;

/// The minimum emission count after an initial value plus one change.
const MIN_EMISSIONS_AFTER_CHANGE: usize = 2;

/// The minimum emission count after an initial value plus two changes.
const MIN_EMISSIONS_AFTER_TWO_CHANGES: usize = 3;

/// The initial city.
const INITIAL_CITY: &str = "Seattle";

/// The replacement city.
const REPLACEMENT_CITY: &str = "Portland";

/// The replacement integer value.
const REPLACEMENT_INT_VALUE: i32 = 2;

/// A view model with `prop1` and `prop2`.
fn with_props(prop1: &str, prop2: i32) -> Rc<BigViewModel> {
    let vm = BigViewModel::new();
    vm.set_prop1(prop1.to_string());
    vm.set_prop2(prop2);
    vm
}

#[test]
fn deep_chain_emits_initial_value() {
    let vm = BigViewModel::new();
    vm.address().set_city(INITIAL_CITY.to_string());

    let values = collect_items(&when_changing_scenarios::deep_chain_address_city(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], INITIAL_CITY);
}

#[test]
fn deep_chain_emits_the_value_each_nested_change_leaves_behind() {
    let vm = BigViewModel::new();
    vm.address().set_city(INITIAL_CITY.to_string());

    let values = collect_items(&when_changing_scenarios::deep_chain_address_city(&vm));

    vm.address().set_city(REPLACEMENT_CITY.to_string());
    vm.address().set_city("Boston".to_string());

    assert!(values.borrow().len() >= MIN_EMISSIONS_AFTER_CHANGE);
    assert_eq!(values.borrow()[0], INITIAL_CITY);
    assert_eq!(values.borrow()[1], REPLACEMENT_CITY);
}

#[test]
fn two_properties_emits_on_either_change() {
    let vm = with_props("A", 1);

    let values = collect_items(&when_changing_scenarios::two_properties(&vm));

    // Initial emission
    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], ("A".to_string(), 1));

    // Change Prop1: should emit before-change values
    vm.set_prop1("B".to_string());

    assert!(values.borrow().len() >= MIN_EMISSIONS_AFTER_CHANGE);
    // The before-change value for Prop1 should still be "A"
    assert_eq!(values.borrow()[1], ("A".to_string(), 1));
}

#[test]
fn deep_chain_disposal_stops_listening() {
    let vm = BigViewModel::new();
    vm.address().set_city(INITIAL_CITY.to_string());
    let values = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = values.clone();
    let subscription = when_changing_scenarios::deep_chain_address_city(&vm).subscribe_next(move |value| sink.borrow_mut().push(value));

    subscription.dispose();

    vm.address().set_city("Portland".to_string());

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], INITIAL_CITY);
}

#[test]
fn two_properties_sequential_changes() {
    let vm = with_props("A", 1);

    let values = collect_items(&when_changing_scenarios::two_properties(&vm));

    vm.set_prop1("B".to_string());
    vm.set_prop2(REPLACEMENT_INT_VALUE);

    // Should have at least 3 emissions: initial + 2 changes
    assert!(values.borrow().len() >= MIN_EMISSIONS_AFTER_TWO_CHANGES);
}
