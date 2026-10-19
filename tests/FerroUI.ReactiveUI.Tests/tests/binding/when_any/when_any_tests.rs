//! `WhenAnyTests` (`ReactiveUI.Binding.Tests/WhenAny/WhenAnyTests.cs`).
//!
//! Upstream calls `WhenAnyUnsafe`, the run-time form; the port has one form, `when_any`, over
//! property paths.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::{NonReactiveINotifyPropertyChangedObject, TestFixture};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::prop;
use std::cell::RefCell;
use std::rc::Rc;

const EXPECTED_TWO_EMISSIONS: usize = 2;

#[test]
fn single_property_with_selector_returns_observed_change() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string("Initial".to_string());

    let values = collect_items(&fixture.when_any(prop!(TestFixture => is_not_null_string), |change| change.value));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], "Initial");

    fixture.set_is_not_null_string("Changed".to_string());

    assert!(values.borrow().len() >= EXPECTED_TWO_EMISSIONS);
    assert_eq!(values.borrow()[1], "Changed");
}

#[test]
fn two_properties_combines_with_selector() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string("Hello".to_string());
    fixture.set_is_only_one_word("World".to_string());

    let values = collect_items(&fixture.when_any(
        (prop!(TestFixture => is_not_null_string), prop!(TestFixture => is_only_one_word)),
        |(c1, c2)| format!("{} {}", c1.value, c2.value),
    ));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], "Hello World");
}

#[test]
fn returns_current_value_on_subscription() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string("PreExisting".to_string());

    let values = collect_items(&fixture.when_any(prop!(TestFixture => is_not_null_string), |change| change.value));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], "PreExisting");
}

#[test]
fn works_with_i_notify_property_changed_objects() {
    let obj = NonReactiveINotifyPropertyChangedObject::new();
    obj.set_inpc_property("Start".to_string());

    let values = collect_items(&obj.when_any(prop!(NonReactiveINotifyPropertyChangedObject => inpc_property), |change| change.value));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], "Start");

    obj.set_inpc_property("End".to_string());

    assert!(values.borrow().len() >= EXPECTED_TWO_EMISSIONS);
}

#[test]
fn exposes_sender_through_observed_change() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string("Test".to_string());
    let captured: Rc<RefCell<Option<ObservedChange<Rc<TestFixture>, String>>>> = Rc::new(RefCell::new(None));

    let slot = captured.clone();
    let _subscription = fixture
        .when_any(prop!(TestFixture => is_not_null_string), |change| change)
        .subscribe_next(move |change| *slot.borrow_mut() = Some(change));

    let captured = captured.borrow();
    let change = captured.as_ref().expect("a change was produced");
    assert!(Rc::ptr_eq(&change.sender, &fixture));
}
