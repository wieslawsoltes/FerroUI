//! Tests of the property diagnostics of objects.

use super::FerroObjectDiagnosticExtensions;
use crate::data::BindingPriority;
use crate::input::InputElement;
use crate::{BoxedValue, StyledElement};
use std::rc::Rc;

fn text(value: &BoxedValue) -> Option<String> {
    value.downcast_ref::<Option<BoxedValue>>()?.as_ref()?.downcast_ref::<String>().cloned()
}

#[test]
fn an_unset_property_reports_its_default_value_without_a_priority() {
    let target = InputElement::new();
    let property = InputElement::is_enabled_property().as_property();
    let diagnostic = target.get_diagnostic(property);
    assert!(std::ptr::eq(diagnostic.property(), property));
    assert_eq!(diagnostic.value().downcast_ref::<bool>(), Some(&true));
    assert_eq!(diagnostic.priority(), BindingPriority::Unset);
    assert_eq!(diagnostic.diagnostic(), None);
    assert!(!diagnostic.is_overridden_current_value());
}

#[test]
fn the_priority_of_the_effective_value_is_reported() {
    let target = InputElement::new();
    let property = InputElement::is_enabled_property();

    target.set_value(property, false);
    let diagnostic = target.get_diagnostic(property.as_property());
    assert_eq!(diagnostic.value().downcast_ref::<bool>(), Some(&false));
    assert_eq!(diagnostic.priority(), BindingPriority::LocalValue);

    let target = InputElement::new();
    let _ = target.set_value_with_priority(property, false, BindingPriority::Style);
    let diagnostic = target.get_diagnostic(property.as_property());
    assert_eq!(diagnostic.value().downcast_ref::<bool>(), Some(&false));
    assert_eq!(diagnostic.priority(), BindingPriority::Style);
    assert!(!diagnostic.is_overridden_current_value());
}

#[test]
fn a_current_value_is_reported_as_overridden() {
    let target = InputElement::new();
    let property = InputElement::is_enabled_property();
    let _ = target.set_value_with_priority(property, true, BindingPriority::Style);
    target.set_current_value(property, false);
    let diagnostic = target.get_diagnostic(property.as_property());
    assert_eq!(diagnostic.value().downcast_ref::<bool>(), Some(&false));
    // The value keeps the priority of the value it overrides.
    assert_eq!(diagnostic.priority(), BindingPriority::Style);
    assert!(diagnostic.is_overridden_current_value());
}

#[test]
fn an_inherited_value_is_reported_with_the_inherited_priority() {
    let parent = StyledElement::new();
    let child = StyledElement::new();
    child.set_inheritance_parent(&parent);
    let property = StyledElement::data_context_property();
    parent.set_value(property, Some(Rc::new("context".to_string()) as BoxedValue));

    let diagnostic = child.get_diagnostic(property.as_property());
    assert_eq!(text(diagnostic.value()).as_deref(), Some("context"));
    assert_eq!(diagnostic.priority(), BindingPriority::Inherited);
    assert!(!diagnostic.is_overridden_current_value());

    let diagnostic = parent.get_diagnostic(property.as_property());
    assert_eq!(diagnostic.priority(), BindingPriority::LocalValue);
}

#[test]
fn a_direct_property_is_reported_as_a_local_value() {
    let target = StyledElement::new();
    target.set_name(Some("target".to_string()));
    let diagnostic = target.get_diagnostic(StyledElement::name_property().as_property());
    assert_eq!(diagnostic.value().downcast_ref::<Option<String>>(), Some(&Some("target".to_string())));
    assert_eq!(diagnostic.priority(), BindingPriority::LocalValue);
    assert!(!diagnostic.is_overridden_current_value());
}
