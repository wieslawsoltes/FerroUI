//! `BindingObservationIntegrationTests` (`ReactiveUI.Tests/WhenAny/BindingObservationIntegrationTests.cs`):
//! the observation of a property path that a generic helper receives.
//!
//! Upstream's helper takes an expression, which its generator cannot read at the call site, and
//! observes it with `WhenAnyValueUnsafe`; here the helper takes a property path, which carries its
//! getter and its names wherever it goes, and observes it with `when_any_value`.

use crate::mocks::{collect_items, WhenAnyArityTestViewModel};
use ferroui_reactiveui::*;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// The expected count after one property change.
const EXPECTED_COUNT_TWO: usize = 2;

#[test]
fn when_any_value_unsafe_generic_helper_delivers_initial_and_changes() {
    let view_model = WhenAnyArityTestViewModel::new();
    let property = prop!(WhenAnyArityTestViewModel => property1);

    let values = collect_items(&observe_from_generic_helper(&view_model, property));
    assert_eq!(values.borrow().len(), 1);
    assert!(values.borrow()[0].is_none());

    view_model.set_property1(Some("updated".to_owned()));

    assert_eq!(values.borrow().len(), EXPECTED_COUNT_TWO);
    assert_eq!(values.borrow()[1].as_deref(), Some("updated"));
}

/// Observes a property path supplied through generic type parameters (`ObserveFromGenericHelper`).
fn observe_from_generic_helper<O: 'static, V: Clone + PartialEq + 'static>(source: &Rc<O>, property: PropertyPath<Rc<O>, V>) -> Rc<dyn IObservable<V>> {
    source.when_any_value(property)
}
