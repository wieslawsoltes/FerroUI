//! Tests of delayed bindings (specific to this port: the upstream class has
//! no tests of its own).

use super::test_support::*;
use crate::data::Binding;
use crate::markup::data::DelayedBinding;
use ferroui_base::data::core::{ClrPropertyInfo, Maybe};
use ferroui_base::*;
use std::rc::Rc;

fn tag_binding() -> Rc<Binding> {
    let binding = Binding::with_path("Tag");
    binding.set_relative_source(Some(ferroui_base::data::RelativeSource::new(ferroui_base::data::RelativeSourceMode::SelfMode)));
    binding
}

#[test]
fn binding_is_applied_when_element_is_initialized() {
    let target = TextBlock::new();
    target.set_tag(bs("foo"));
    assert!(!target.is_initialized());

    DelayedBinding::add(&target, TextBlock::text_property().as_property(), tag_binding());
    assert!(target.text().is_none());

    target.initialize_if_needed();

    assert!(target.is_initialized());
    assert_eq!(target.text().as_deref(), Some("foo"));
}

#[test]
fn binding_is_applied_immediately_to_initialized_element() {
    let target = TextBlock::new();
    target.set_tag(bs("foo"));
    target.initialize_if_needed();

    DelayedBinding::add(&target, TextBlock::text_property().as_property(), tag_binding());

    assert_eq!(target.text().as_deref(), Some("foo"));
}

#[test]
fn apply_bindings_applies_pending_bindings_once() {
    let target = TextBlock::new();
    target.set_tag(bs("foo"));

    DelayedBinding::add(&target, TextBlock::text_property().as_property(), tag_binding());
    DelayedBinding::apply_bindings(&target);
    assert_eq!(target.text().as_deref(), Some("foo"));

    // The pending entries were consumed: initializing does not bind again.
    target.set_value(TextBlock::text_property(), Some(s("local")));
    target.initialize_if_needed();
    assert_eq!(target.text().as_deref(), Some("local"));
}

#[test]
fn value_is_set_when_element_is_initialized() {
    let target = TextBlock::new();
    let property: Rc<ClrPropertyInfo> = Rc::new(ClrPropertyInfo::read_write::<Ref<FerroObject>, Maybe<String>>(
        "Text",
        |o| o.cast::<TextBlock>().and_then(|t| t.text()),
        |o, v| {
            if let Some(t) = o.cast::<TextBlock>() {
                t.set_text(v.as_deref());
            }
        },
    ));

    DelayedBinding::add_value(&target, property.clone(), |_| bs("delayed")).unwrap();
    assert!(target.text().is_none());

    target.initialize_if_needed();
    assert_eq!(target.text().as_deref(), Some("delayed"));

    DelayedBinding::add_value(&target, property, |_| bs("immediate")).unwrap();
    assert_eq!(target.text().as_deref(), Some("immediate"));
}
