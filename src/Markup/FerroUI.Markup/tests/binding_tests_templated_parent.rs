//! Ported from the upstream `BindingTests_TemplatedParent`.
//!
//! The upstream tests build the template of a button; here the template
//! child is created with its binding and then attached to the templated
//! parent in the order in which a templated control applies its template.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::{BindingMode, RelativeSource, RelativeSourceMode};
use ferroui_base::Ref;

fn build(mode: BindingMode) -> (Ref<Button>, Ref<ContentPresenter>) {
    let source = Button::new();
    let target = ContentPresenter::new();
    let binding = Binding::with_path_and_mode("Content", mode);
    binding.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent)));
    target.bind_binding(ContentPresenter::content_property(), &binding);
    apply_template(&source, &target);
    (source, target)
}

#[test]
fn one_way_binding_should_be_set_up() {
    let (source, target) = build(BindingMode::OneWay);

    assert!(target.content().is_none());
    source.set_content(bs("foo"));
    assert_eq!(as_string(&target.content()).as_deref(), Some("foo"));
    source.set_content(bs("bar"));
    assert_eq!(as_string(&target.content()).as_deref(), Some("bar"));
}

#[test]
fn two_way_binding_should_be_set_up() {
    let (source, target) = build(BindingMode::TwoWay);

    assert!(target.content().is_none());
    source.set_content(bs("foo"));
    assert_eq!(as_string(&target.content()).as_deref(), Some("foo"));
    target.set_content(bs("bar"));
    assert_eq!(as_string(&source.content()).as_deref(), Some("bar"));
}
