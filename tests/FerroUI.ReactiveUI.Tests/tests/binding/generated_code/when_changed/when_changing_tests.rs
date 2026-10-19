//! `WhenChangingTests` (`ReactiveUI.Binding.GeneratedCode.Tests/WhenChanged/WhenChangingTests.cs`).
//!
//! `WhenChanging` produces the current values on subscription, then the value before each change,
//! repeats included.

use crate::mocks::collect_items;
use crate::tests::binding::generated_code::test_models::scenarios::when_changing_scenarios;
use crate::tests::binding::generated_code::test_models::{BigViewModel, TestViewModel};
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;

/// The initial name value.
const INITIAL_NAME: &str = "Initial";

/// The integer value of the two-property test.
const TWO_PROP_INT_VALUE: i32 = 42;

/// The integer value of the three-property test.
const THREE_PROP_INT_VALUE: i32 = 10;

/// The integer value of the four-property test.
const FOUR_PROP_INT_VALUE: i32 = 20;

/// The double value of the three-property test.
#[allow(clippy::approx_constant)]
const THREE_PROP_DOUBLE_VALUE: f64 = 3.14;

/// The double value of the four-property test.
const FOUR_PROP_DOUBLE_VALUE: f64 = 2.71;

/// The value before the change.
const BEFORE_VALUE: &str = "Before";

/// A view model named `name`.
fn named(name: &str) -> Rc<TestViewModel> {
    let vm = TestViewModel::new();
    vm.set_name(name.to_string());
    vm
}

#[test]
fn single_property_emits_initial_value() {
    let vm = named(INITIAL_NAME);

    let values = collect_items(&when_changing_scenarios::single_property_name(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], INITIAL_NAME);
}

#[test]
fn single_property_emits_before_change() {
    let vm = named(BEFORE_VALUE);

    let values = collect_items(&when_changing_scenarios::single_property_name(&vm));

    vm.set_name("After".to_string());

    // WhenChanging emits the value at the time of the PropertyChanging event, which is the old
    // value (before the assignment).
    assert_eq!(values.borrow()[0], BEFORE_VALUE);
    assert_eq!(*values.borrow(), vec![BEFORE_VALUE.to_string(), BEFORE_VALUE.to_string()]);
}

#[test]
fn single_property_emits_sequential_before_change_values() {
    let vm = named("A");

    let values = collect_items(&when_changing_scenarios::single_property_name(&vm));

    vm.set_name("B".to_string());
    vm.set_name("C".to_string());

    // Initial "A", "A" before the change to "B", then "B" before the change to "C".
    assert_eq!(*values.borrow(), vec!["A".to_string(), "A".to_string(), "B".to_string()]);
}

#[test]
fn two_properties_emits_initial_tuple() {
    let vm = BigViewModel::new();
    vm.set_prop1("Hello".to_string());
    vm.set_prop2(TWO_PROP_INT_VALUE);

    let values = collect_items(&when_changing_scenarios::two_properties(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0].0, "Hello");
    assert_eq!(values.borrow()[0].1, TWO_PROP_INT_VALUE);
}

#[test]
fn three_properties_emits_initial_values() {
    let vm = BigViewModel::new();
    vm.set_prop1("X".to_string());
    vm.set_prop2(THREE_PROP_INT_VALUE);
    vm.set_prop3(THREE_PROP_DOUBLE_VALUE);

    let values = collect_items(&when_changing_scenarios::three_properties(&vm));

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

    let values = collect_items(&when_changing_scenarios::four_properties(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0].0, "Y");
    assert_eq!(values.borrow()[0].1, FOUR_PROP_INT_VALUE);
    assert_eq!(values.borrow()[0].2, FOUR_PROP_DOUBLE_VALUE);
    assert!(values.borrow()[0].3);
}

#[test]
fn disposal_stops_listening() {
    let vm = named(INITIAL_NAME);
    let values = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = values.clone();
    let subscription = when_changing_scenarios::single_property_name(&vm).subscribe_next(move |value| sink.borrow_mut().push(value));

    subscription.dispose();

    vm.set_name("AfterDisposal".to_string());

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], INITIAL_NAME);
}
