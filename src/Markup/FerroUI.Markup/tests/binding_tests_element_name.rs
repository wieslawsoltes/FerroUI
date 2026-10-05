//! Ported from the upstream `BindingTests_ElementName`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::*;

fn text_block(name: &str, text: Option<&str>) -> Ref<TextBlock> {
    let result = TextBlock::new();
    result.set_name(Some(s(name)));
    if text.is_some() {
        result.set_text(text);
    }
    result
}

fn content_control(name: &str) -> Ref<ContentControl> {
    let result = ContentControl::new();
    result.set_name(Some(s(name)));
    result
}

fn is_same_object<T: ObjectType>(value: &Option<BoxedValue>, expected: &Ref<T>) -> bool {
    value.as_ref().and_then(|v| ValueTypes::as_object(&**v)).is_some_and(|o| o.ptr_eq(expected))
}

#[test]
fn should_bind_to_element_path() {
    let target = text_block("target", None);
    let stack_panel = StackPanel::new();
    stack_panel.add(&text_block("source", Some("foo")));
    stack_panel.add(&target);
    let root = TestRoot::with_child(&stack_panel);

    root.register_children_names();

    let binding = Binding::with_path("Text");
    binding.set_element_name(Some(s("source")));
    binding.set_name_scope(root.name_scope());

    target.bind_binding(TextBox::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("foo"));
}

#[test]
fn should_bind_to_element() {
    let source = text_block("source", Some("foo"));
    let target = content_control("target");
    let stack_panel = StackPanel::new();
    stack_panel.add(&source);
    stack_panel.add(&target);
    let root = TestRoot::with_child(&stack_panel);

    root.register_children_names();

    let binding = Binding::new();
    binding.set_element_name(Some(s("source")));
    binding.set_name_scope(root.name_scope());

    target.bind_binding(ContentControl::content_property(), &binding);

    assert!(is_same_object(&target.content(), &source));
}

#[test]
fn should_bind_to_later_added_element_path() {
    let target = text_block("target", None);
    let stack_panel = StackPanel::new();
    stack_panel.add(&target);
    let root = TestRoot::with_child(&stack_panel);

    root.register_children_names();

    let binding = Binding::with_path("Text");
    binding.set_element_name(Some(s("source")));
    binding.set_name_scope(root.name_scope());

    target.bind_binding(TextBox::text_property(), &binding);

    stack_panel.add(&text_block("source", Some("foo")));
    root.register_children_names();

    assert_eq!(target.text().as_deref(), Some("foo"));
}

#[test]
fn should_bind_to_later_added_element() {
    let target = content_control("target");
    let stack_panel = StackPanel::new();
    stack_panel.add(&target);
    let root = TestRoot::with_child(&stack_panel);

    root.register_children_names();

    let binding = Binding::new();
    binding.set_element_name(Some(s("source")));
    binding.set_name_scope(root.name_scope());

    target.bind_binding(ContentControl::content_property(), &binding);

    let source = text_block("source", Some("foo"));
    stack_panel.add(&source);
    root.register_children_names();

    assert!(is_same_object(&target.content(), &source));
}
