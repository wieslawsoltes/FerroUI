//! Ported from the upstream `BindingTests_Converters`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::converters::{IValueConverter, StringConverters};
use ferroui_base::data::core::{BindingExpression, Value, ValueType};
use ferroui_base::data::model::Model;
use ferroui_base::data::{BindingBase, BindingError, BindingMode};
use ferroui_base::utilities::CultureInfo;
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

/// One call of a converter: the method, the text of the value, the target type, whether
/// there was a parameter, and the culture.
type ConverterCall = (&'static str, Option<String>, ValueType, bool, CultureInfo);

/// The mock of the upstream tests: records its calls and returns null.
#[derive(Default)]
struct RecordingConverter {
    calls: RefCell<Vec<ConverterCall>>,
}

impl RecordingConverter {
    fn record(
        &self,
        method: &'static str,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) {
        let text = value.and_then(|value| value.downcast_ref::<String>().cloned());
        self.calls.borrow_mut().push((method, text, target_type, parameter.is_some(), culture.clone()));
    }

    fn calls_of(&self, method: &'static str) -> Vec<ConverterCall> {
        self.calls.borrow().iter().filter(|call| call.0 == method).cloned().collect()
    }
}

impl IValueConverter for RecordingConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        self.record("Convert", value, target_type, parameter, culture);
        Ok(None)
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        self.record("ConvertBack", value, target_type, parameter, culture);
        Ok(None)
    }
}

#[test]
fn converter_culture_should_be_passed_to_converter_convert() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let culture = CultureInfo::get_culture_info("ar-SA");
    let converter = Rc::new(RecordingConverter::default());
    let target = Binding::with_path("Foo");
    target.set_converter(Some(converter.clone()));
    target.set_converter_culture(Some(culture.clone()));

    text_block.bind_binding(TextBlock::text_property(), &target);

    let expected: ConverterCall = ("Convert", Some(s("foo")), ValueType::of::<Option<String>>(), false, culture);
    assert_eq!(converter.calls_of("Convert"), [expected]);
}

#[test]
fn converter_culture_should_be_passed_to_converter_convert_back() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let culture = CultureInfo::get_culture_info("ar-SA");
    let converter = Rc::new(RecordingConverter::default());
    let target = Binding::with_path("Foo");
    target.set_converter(Some(converter.clone()));
    target.set_converter_culture(Some(culture.clone()));
    target.set_mode(BindingMode::TwoWay);

    text_block.bind_binding(TextBlock::text_property(), &target);
    text_block.set_text(Some("bar"));

    let expected: ConverterCall = ("ConvertBack", Some(s("bar")), ValueType::of::<String>(), false, culture);
    assert_eq!(converter.calls_of("ConvertBack"), [expected]);
}
