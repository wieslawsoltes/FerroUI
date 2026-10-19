//! `ObservedChangedExtensionTests` (`ReactiveUI.Binding.Tests/ObservedChanged/ObservedChangedExtensionTests.cs`).
//!
//! Upstream builds the expression of a change with `Reflection.Rewrite`; a change of the port
//! carries the text of a property path (`PropertyPath::path`), which is what
//! `GetPropertyName` reads. Upstream's `EnsureInitialized` builds the application with the core
//! services; the port observes with the default observers while none is registered.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::{HostTestFixture, TestFixture};
use ferroui_reactiveui::*;
use ferroui_system::prop;
use std::rc::Rc;

/// The expected number of emissions after the initial value plus one change.
const EXPECTED_TWO_EMISSIONS: usize = 2;

#[test]
fn value_converts_observed_changes_to_values() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string("Start".to_string());

    let values = collect_items(&fixture.observable_for_property_skip_initial(prop!(TestFixture => is_not_null_string), false).value());

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], "Start");

    fixture.set_is_not_null_string("End".to_string());

    assert!(values.borrow().len() >= EXPECTED_TWO_EMISSIONS);
    assert_eq!(values.borrow()[1], "End");
}

#[test]
fn get_property_name_returns_correct_path() {
    let expression = prop!(TestFixture => is_not_null_string).path();
    let change = ObservedChange::new(TestFixture::new(), expression, "value".to_string());

    let name = change.get_property_name();
    assert_eq!(name, "IsNotNullString");
}

#[test]
fn get_property_name_nested_returns_dotted_path() {
    let expression = prop!(HostTestFixture => child)
        .then_optional_property("IsOnlyOneWord", |child: &Rc<TestFixture>| child.is_only_one_word())
        .path();
    let change = ObservedChange::new(HostTestFixture::new(), expression, "value".to_string());

    let name = change.get_property_name();
    assert_eq!(name, "Child.IsOnlyOneWord");
}

// Not ported: GetValue_EvaluatesExpressionChain - expression trees (upstream reads the value again
// through the expression when the change carries the default; a change of the port always carries
// its value).
// Not ported: GetValueOrDefault_NullChain_ReturnsDefault - expression trees (same reason).
