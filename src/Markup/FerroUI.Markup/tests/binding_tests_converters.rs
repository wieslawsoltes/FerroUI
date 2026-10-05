//! Ported from the upstream `BindingTests_Converters`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::converters::StringConverters;
use ferroui_base::data::core::{BindingExpression, Value};
use ferroui_base::data::model::Model;
use ferroui_base::data::BindingBase;
use ferroui_base::*;
use std::cell::RefCell;
use std::rc::Rc;

struct Class1 {
    foo: RefCell<String>,
}

impl Class1 {
    fn new() -> Rc<Self> {
        Model::new_model(Self { foo: RefCell::new(s("foo")) })
    }
}

ferro_model!(Class1, |b| b.property::<Value<String>>("Foo", |o| o.foo.borrow().clone(), |o, v| {
    o.foo.replace(v);
}));

#[test]
fn converter_should_be_used() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let target = Binding::with_path("Foo");
    target.set_converter(Some(StringConverters::is_null_or_empty()));

    let expression = target.create_instance(&text_block, Some(TextBlock::text_property().as_property()), None);
    let expression = expression.as_any().downcast_ref::<BindingExpression>().expect("a binding expression");

    assert!(Rc::ptr_eq(&StringConverters::is_null_or_empty(), expression.converter().expect("a converter")));
}

#[test]
fn string_format_should_be_applied() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let target = Binding::with_path("Foo");
    target.set_string_format(Some(s("Hello {0}")));

    text_block.bind_binding(TextBlock::text_property(), &target);

    assert_eq!(text_block.text().as_deref(), Some("Hello foo"));
}

#[test]
fn string_format_should_be_applied_after_converter() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let target = Binding::with_path("Foo");
    target.set_converter(Some(StringConverters::is_not_null_or_empty()));
    target.set_string_format(Some(s("Hello {0}")));

    text_block.bind_binding(TextBlock::text_property(), &target);

    assert_eq!(text_block.text().as_deref(), Some("Hello True"));
}
