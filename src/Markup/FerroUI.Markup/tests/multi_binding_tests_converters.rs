//! Ported from the upstream `MultiBindingTests_Converters`.

use ferroui_base::utilities::CultureInfo;
use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::converters::IMultiValueConverter;
use ferroui_base::data::core::{Value, ValueType};
use ferroui_base::data::model::Model;
use ferroui_base::data::{BindingBase, BindingError, MultiBinding};
use ferroui_base::layout::Layoutable;
use ferroui_base::*;
use std::cell::Cell;
use std::rc::Rc;

struct SumOfDoublesConverter;

impl IMultiValueConverter for SumOfDoublesConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let sum: f64 = values.iter().flatten().filter_map(|v| v.downcast_ref::<f64>()).sum();
        Ok(Some(boxed(sum)))
    }
}

struct Class1 {
    foo: Cell<f64>,
    bar: Cell<f64>,
}

impl Class1 {
    fn new() -> Rc<Self> {
        Model::new_model(Self { foo: Cell::new(1.0), bar: Cell::new(2.0) })
    }
}

ferro_model!(Class1, |b| b
    .property::<Value<f64>>("Foo", |o| o.foo.get(), |o, v| o.foo.set(v))
    .property::<Value<f64>>("Bar", |o| o.bar.get(), |o, v| o.bar.set(v)));

fn bindings() -> Vec<Rc<dyn BindingBase>> {
    vec![Binding::with_path("Foo"), Binding::with_path("Bar")]
}

#[test]
fn string_format_should_be_applied() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let format = "{0:0.0} + {1:00}";
    let target = MultiBinding::new().with_string_format(Some(s(format))).with_bindings(bindings());

    text_block.bind_binding(TextBlock::text_property(), &target);

    assert_eq!(text_block.text().as_deref(), Some("1.0 + 02"));
}

#[test]
fn string_format_should_be_applied_after_converter() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let target = MultiBinding::new().with_string_format(Some(s("Foo + Bar = {0}"))).with_converter_value(Some(Rc::new(SumOfDoublesConverter))).with_bindings(bindings());

    text_block.bind_binding(TextBlock::text_property(), &target);

    assert_eq!(text_block.text().as_deref(), Some("Foo + Bar = 3"));
}

#[test]
fn string_format_should_not_be_applied_when_binding_to_non_string_or_object() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let target = MultiBinding::new().with_string_format(Some(s("Hello {0}"))).with_converter_value(Some(Rc::new(SumOfDoublesConverter))).with_bindings(bindings());

    text_block.bind_binding(Layoutable::width_property(), &target);

    assert_eq!(text_block.width(), 3.0);
}
