//! `WhenAnyValueTests` (`ReactiveUI.Binding.GeneratedCode.Tests/WhenAnyValue/WhenAnyValueTests.cs`).
//!
//! Upstream's `PropertyValues<T1, ..., Tn>` is a tuple: `Property1` is `.0`.

use crate::mocks::collect_items;
use crate::tests::binding::generated_code::test_models::scenarios::when_any_value_scenarios;
use crate::tests::binding::generated_code::test_models::WhenAnyTestFixture;

/// The expected emission count after an initial value plus one change.
const EXPECTED_EMISSION_COUNT: usize = 2;

/// The second emission index.
const SECOND_INDEX: usize = 2;

/// The expected emission count after an initial value plus three changes.
const FOUR_EMISSION_COUNT: usize = 4;

/// The third emission index.
const THIRD_INDEX: usize = 3;

#[test]
fn single_property_emits_initial_and_changes() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1("A".to_string());

    let values = collect_items(&when_any_value_scenarios::single_property(&fixture));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], "A");

    fixture.set_value1("B".to_string());

    assert!(values.borrow().len() >= EXPECTED_EMISSION_COUNT);
    assert!(values.borrow().contains(&"B".to_string()));
}

#[test]
fn two_properties_emits_tuples() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1("A".to_string());
    fixture.set_value2("B".to_string());

    let values = collect_items(&when_any_value_scenarios::two_properties(&fixture));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0].0, "A");
    assert_eq!(values.borrow()[0].1, "B");
}

#[test]
fn three_properties_emits_tuples() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1("A".to_string());
    fixture.set_value2("B".to_string());
    fixture.set_value3("C".to_string());

    let values = collect_items(&when_any_value_scenarios::three_properties(&fixture));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0].0, "A");
    assert_eq!(values.borrow()[0].1, "B");
    assert_eq!(values.borrow()[0].2, "C");
}

#[test]
fn with_selector_projects_values() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1("Hello".to_string());
    fixture.set_value2("World".to_string());

    let values = collect_items(&when_any_value_scenarios::with_selector_two_properties(&fixture));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], "Hello_World");
}

#[test]
fn sequential_changes_emits_all() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1("A".to_string());

    let values = collect_items(&when_any_value_scenarios::single_property(&fixture));

    fixture.set_value1("B".to_string());
    fixture.set_value1("C".to_string());
    fixture.set_value1("D".to_string());

    assert!(values.borrow().len() >= FOUR_EMISSION_COUNT);
    assert_eq!(values.borrow()[0], "A");
    assert_eq!(values.borrow()[1], "B");
    assert_eq!(values.borrow()[SECOND_INDEX], "C");
    assert_eq!(values.borrow()[THIRD_INDEX], "D");
}
