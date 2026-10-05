//! Ported from the upstream `BindingTests_Self`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::{BindingMode, RelativeSource, RelativeSourceMode};

#[test]
fn binding_to_property_on_self_should_work() {
    let target = TextBlock::new();
    target.set_tag(bs("Hello World!"));
    let binding = Binding::with_path("Tag");
    binding.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::SelfMode)));
    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("Hello World!"));
}

#[test]
fn two_way_binding_to_property_on_self_should_work() {
    let target = TextBlock::new();
    target.set_tag(bs("Hello World!"));
    let binding = Binding::with_path_and_mode("Tag", BindingMode::TwoWay);
    binding.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::SelfMode)));
    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("Hello World!"));
    target.set_text(Some("Goodbye cruel world :("));
    assert_eq!(target.text().as_deref(), Some("Goodbye cruel world :("));
}
