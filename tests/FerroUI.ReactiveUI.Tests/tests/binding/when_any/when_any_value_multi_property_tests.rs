//! `WhenAnyValueMultiPropertyTests` (`ReactiveUI.Binding.Tests/WhenAny/WhenAnyValueMultiPropertyTests.cs`).
//!
//! Upstream calls `WhenAnyValueUnsafe` with 1 to 12 expressions; the port takes one property path
//! or a tuple of them, and upstream's `PropertyValues<T1, ..., Tn>` is a tuple (`Property1` is
//! `.0`). Upstream's `EnsureInitialized` builds the application with the core services; the port
//! observes with the default observers while none is registered, so the tests need no setup.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::{HostTestFixture, TestFixture, WhenAnyTestFixture};
use ferroui_reactiveui::*;
use ferroui_system::prop;
use std::rc::Rc;

/// The expected number of emissions after three successive value changes.
const EXPECTED_THREE_EMISSIONS: usize = 3;

/// The index of the third emitted value.
const THIRD_VALUE_INDEX: usize = 2;

/// The expected number of emissions after a deep-chain value change.
const EXPECTED_TWO_EMISSIONS: usize = 2;

/// A fixture whose `Value1`, `Value2`, ... hold `values`, in order.
fn fixture_with(values: &[&str]) -> Rc<WhenAnyTestFixture> {
    let fixture = WhenAnyTestFixture::new();
    let setters: [fn(&WhenAnyTestFixture, String); 12] = [
        WhenAnyTestFixture::set_value1,
        WhenAnyTestFixture::set_value2,
        WhenAnyTestFixture::set_value3,
        WhenAnyTestFixture::set_value4,
        WhenAnyTestFixture::set_value5,
        WhenAnyTestFixture::set_value6,
        WhenAnyTestFixture::set_value7,
        WhenAnyTestFixture::set_value8,
        WhenAnyTestFixture::set_value9,
        WhenAnyTestFixture::set_value10,
        WhenAnyTestFixture::set_value11,
        WhenAnyTestFixture::set_value12,
    ];
    for (setter, value) in setters.iter().zip(values) {
        setter(&*fixture, value.to_string());
    }
    fixture
}

#[test]
fn when_any_value_1_property() {
    let fixture = fixture_with(&["A"]);

    let values = collect_items(&fixture.when_any_value(prop!(WhenAnyTestFixture => value1)));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], "A");
}

#[test]
fn when_any_value_2_properties() {
    let fixture = fixture_with(&["A", "B"]);

    let values = collect_items(&fixture.when_any_value((prop!(WhenAnyTestFixture => value1), prop!(WhenAnyTestFixture => value2))));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0].0, "A");
    assert_eq!(values.borrow()[0].1, "B");
}

#[test]
fn when_any_value_3_properties() {
    let fixture = fixture_with(&["A", "B", "C"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
    )));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0].0, "A");
    assert_eq!(values.borrow()[0].1, "B");
    assert_eq!(values.borrow()[0].2, "C");
}

#[test]
fn when_any_value_4_properties() {
    let fixture = fixture_with(&["A", "B", "C", "D"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
    )));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0].0, "A");
    assert_eq!(values.borrow()[0].3, "D");
}

#[test]
fn when_any_value_5_properties() {
    let fixture = fixture_with(&["A", "B", "C", "D", "E"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
        prop!(WhenAnyTestFixture => value5),
    )));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0].0, "A");
    assert_eq!(values.borrow()[0].4, "E");
}

#[test]
fn when_any_value_6_properties() {
    let fixture = fixture_with(&["A", "B", "C", "D", "E", "F"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
        prop!(WhenAnyTestFixture => value5),
        prop!(WhenAnyTestFixture => value6),
    )));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0].0, "A");
    assert_eq!(values.borrow()[0].5, "F");
}

#[test]
fn when_any_value_7_properties() {
    let fixture = fixture_with(&["A", "B", "C", "D", "E", "F", "G"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
        prop!(WhenAnyTestFixture => value5),
        prop!(WhenAnyTestFixture => value6),
        prop!(WhenAnyTestFixture => value7),
    )));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0].6, "G");
}

#[test]
fn when_any_value_8_properties() {
    let fixture = fixture_with(&["A", "B", "C", "D", "E", "F", "G", "H"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
        prop!(WhenAnyTestFixture => value5),
        prop!(WhenAnyTestFixture => value6),
        prop!(WhenAnyTestFixture => value7),
        prop!(WhenAnyTestFixture => value8),
    )));

    let last_item8 = values.borrow().last().map(|values| values.7.clone());
    assert_eq!(last_item8.as_deref(), Some("H"));
}

#[test]
fn when_any_value_9_properties() {
    let fixture = fixture_with(&["A", "B", "C", "D", "E", "F", "G", "H", "I"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
        prop!(WhenAnyTestFixture => value5),
        prop!(WhenAnyTestFixture => value6),
        prop!(WhenAnyTestFixture => value7),
        prop!(WhenAnyTestFixture => value8),
        prop!(WhenAnyTestFixture => value9),
    )));

    let last_item9 = values.borrow().last().map(|values| values.8.clone());
    assert_eq!(last_item9.as_deref(), Some("I"));
}

#[test]
fn when_any_value_10_properties() {
    let fixture = fixture_with(&["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
        prop!(WhenAnyTestFixture => value5),
        prop!(WhenAnyTestFixture => value6),
        prop!(WhenAnyTestFixture => value7),
        prop!(WhenAnyTestFixture => value8),
        prop!(WhenAnyTestFixture => value9),
        prop!(WhenAnyTestFixture => value10),
    )));

    let last_item10 = values.borrow().last().map(|values| values.9.clone());
    assert_eq!(last_item10.as_deref(), Some("J"));
}

#[test]
fn when_any_value_11_properties() {
    let fixture = fixture_with(&["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
        prop!(WhenAnyTestFixture => value5),
        prop!(WhenAnyTestFixture => value6),
        prop!(WhenAnyTestFixture => value7),
        prop!(WhenAnyTestFixture => value8),
        prop!(WhenAnyTestFixture => value9),
        prop!(WhenAnyTestFixture => value10),
        prop!(WhenAnyTestFixture => value11),
    )));

    let last_item11 = values.borrow().last().map(|values| values.10.clone());
    assert_eq!(last_item11.as_deref(), Some("K"));
}

#[test]
fn when_any_value_12_properties() {
    let fixture = fixture_with(&["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L"]);

    let values = collect_items(&fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
        prop!(WhenAnyTestFixture => value5),
        prop!(WhenAnyTestFixture => value6),
        prop!(WhenAnyTestFixture => value7),
        prop!(WhenAnyTestFixture => value8),
        prop!(WhenAnyTestFixture => value9),
        prop!(WhenAnyTestFixture => value10),
        prop!(WhenAnyTestFixture => value11),
        prop!(WhenAnyTestFixture => value12),
    )));

    let last_item12 = values.borrow().last().map(|values| values.11.clone());
    assert_eq!(last_item12.as_deref(), Some("L"));
}

#[test]
fn when_any_value_1_property_sequential_changes() {
    let fixture = fixture_with(&["A"]);

    let values = collect_items(&fixture.when_any_value(prop!(WhenAnyTestFixture => value1)));

    fixture.set_value1("B".to_string());
    fixture.set_value1("C".to_string());

    assert!(values.borrow().len() >= EXPECTED_THREE_EMISSIONS);
    assert_eq!(values.borrow()[0], "A");
    assert_eq!(values.borrow()[1], "B");
    assert_eq!(values.borrow()[THIRD_VALUE_INDEX], "C");
}

#[test]
fn when_any_value_2_properties_with_selector() {
    let fixture = fixture_with(&["Hello", "World"]);

    let values = collect_items(&fixture.when_any_value_select(
        (prop!(WhenAnyTestFixture => value1), prop!(WhenAnyTestFixture => value2)),
        |(v1, v2)| format!("{v1} {v2}"),
    ));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], "Hello World");
}

#[test]
fn when_any_value_deep_chain() {
    let child = TestFixture::new();
    child.set_is_not_null_string("Deep".to_string());
    let fixture = HostTestFixture::new();
    fixture.set_child(Some(child.clone()));

    // `x => x.Child!.IsNotNullString`: the value is `None` while there is no child.
    let values = collect_items(
        &fixture.when_any_value(
            prop!(HostTestFixture => child).then_optional_property("IsNotNullString", |child: &Rc<TestFixture>| child.is_not_null_string()),
        ),
    );

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0].as_deref(), Some("Deep"));

    child.set_is_not_null_string("Deeper".to_string());

    assert!(values.borrow().len() >= EXPECTED_TWO_EMISSIONS);
    assert_eq!(values.borrow()[1].as_deref(), Some("Deeper"));
}
