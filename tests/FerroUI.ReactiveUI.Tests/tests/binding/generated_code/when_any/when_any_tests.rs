//! `WhenAnyTests` (`ReactiveUI.Binding.GeneratedCode.Tests/WhenAny/WhenAnyTests.cs`).

use crate::mocks::collect_items;
use crate::tests::binding::generated_code::test_models::scenarios::when_any_scenarios;
use crate::tests::binding::generated_code::test_models::TestViewModel;
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;

/// The initial name value used across the WhenAny tests.
const INITIAL_NAME: &str = "Initial";

/// The expected emission count after an initial value plus one change.
const EXPECTED_EMISSION_COUNT: usize = 2;

/// The replacement age value used in change-emission tests.
const INITIAL_AGE: i32 = 25;

/// The age used by the two-property fixtures in these tests.
const ADULT_AGE: i32 = 30;

/// A view model with `name` and `age`.
fn view_model(name: &str, age: i32) -> Rc<TestViewModel> {
    let vm = TestViewModel::new();
    vm.set_name(name.to_string());
    vm.set_age(age);
    vm
}

#[test]
fn single_property_emits_initial_value() {
    let vm = view_model(INITIAL_NAME, 0);

    let values = collect_items(&when_any_scenarios::single_property_name(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], INITIAL_NAME);
}

#[test]
fn single_property_emits_on_change() {
    let vm = view_model(INITIAL_NAME, 0);

    let values = collect_items(&when_any_scenarios::single_property_name(&vm));

    vm.set_name("Changed".to_string());

    assert!(values.borrow().len() >= EXPECTED_EMISSION_COUNT);
    assert!(values.borrow().contains(&"Changed".to_string()));
}

#[test]
fn single_property_i_observed_change_has_sender_and_value() {
    let vm = view_model("Hello", 0);

    let changes = collect_items(&when_any_scenarios::single_property_name_observed_change(&vm));

    assert!(!changes.borrow().is_empty());
    assert_eq!(changes.borrow()[0].value, "Hello");
    assert!(Rc::ptr_eq(&changes.borrow()[0].sender, &vm));
}

#[test]
fn two_properties_emits_initial_combined_value() {
    let vm = view_model("Alice", ADULT_AGE);

    let values = collect_items(&when_any_scenarios::two_properties_name_age(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], "Alice_30");
}

#[test]
fn two_properties_emits_on_either_change() {
    let vm = view_model("Alice", ADULT_AGE);

    let values = collect_items(&when_any_scenarios::two_properties_name_age(&vm));

    vm.set_name("Bob".to_string());

    assert_eq!(values.borrow().last().map(String::as_str), Some("Bob_30"));

    vm.set_age(INITIAL_AGE);

    assert_eq!(values.borrow().last().map(String::as_str), Some("Bob_25"));
}

#[test]
fn disposal_stops_listening() {
    let vm = view_model(INITIAL_NAME, 0);
    let values = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = values.clone();
    let subscription = when_any_scenarios::single_property_name(&vm).subscribe_next(move |value| sink.borrow_mut().push(value));

    subscription.dispose();

    vm.set_name("AfterDisposal".to_string());

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], INITIAL_NAME);
}
