//! Port of `Converters/FerroPropertyConverterTest.cs`.

use std::rc::Rc;

use ferroui_base::styling::{Selectors, Style};
use ferroui_base::{BoxedValue, FerroProperty, Ref};
use ferroui_markup_xaml::converters::{FerroPropertyTypeConverter, ITypeDescriptorContext, TypeConverter};

use crate::support::app::{xaml_test_base, XamlTestScope};
use crate::support::converters::ferro_property_converter_test::{AttachedOwner, Class1, TestTypeDescriptorContext};
use crate::support::helpers::{boxed, value_of};

/// The constructor of the test class.
fn ferro_property_converter_test() -> XamlTestScope {
    let base = xaml_test_base();

    // Ensure properties are registered.
    let _ = Class1::foo_property();
    let _ = AttachedOwner::attached_property();

    base
}

fn class1_style() -> Ref<Style> {
    let style = Style::new();
    style.set_selector(Some(Selectors::of_type::<Class1>()));
    style
}

fn create_context(style: Option<Ref<Style>>) -> Rc<dyn ITypeDescriptorContext> {
    TestTypeDescriptorContext::new(style.into_iter().map(|style| -> BoxedValue { boxed(style) }).collect())
}

#[test]
fn convert_from_finds_fully_qualified_property() {
    let _base = ferro_property_converter_test();
    let target = FerroPropertyTypeConverter::new();
    let style = class1_style();
    let context = create_context(Some(style));
    let result = target.convert_from(Some(&context), None, Some(&boxed("Class1.Foo".to_string()))).unwrap();

    let expected: &'static FerroProperty = Class1::foo_property();
    assert!(Some(expected) == value_of::<&'static FerroProperty>(&result));
}

#[test]
fn convert_from_uses_selector_target_type() {
    let _base = ferro_property_converter_test();
    let target = FerroPropertyTypeConverter::new();
    let style = class1_style();
    let context = create_context(Some(style));
    let result = target.convert_from(Some(&context), None, Some(&boxed("Foo".to_string()))).unwrap();

    let expected: &'static FerroProperty = Class1::foo_property();
    assert!(Some(expected) == value_of::<&'static FerroProperty>(&result));
}

#[test]
fn convert_from_finds_attached_property() {
    let _base = ferro_property_converter_test();
    let target = FerroPropertyTypeConverter::new();
    let style = class1_style();
    let context = create_context(Some(style));
    let result = target.convert_from(Some(&context), None, Some(&boxed("AttachedOwner.Attached".to_string()))).unwrap();

    let expected: &'static FerroProperty = AttachedOwner::attached_property();
    assert!(Some(expected) == value_of::<&'static FerroProperty>(&result));
}

#[test]
fn convert_from_finds_attached_property_with_parentheses() {
    let _base = ferro_property_converter_test();
    let target = FerroPropertyTypeConverter::new();
    let style = class1_style();
    let context = create_context(Some(style));
    let result =
        target.convert_from(Some(&context), None, Some(&boxed("(AttachedOwner.Attached)".to_string()))).unwrap();

    let expected: &'static FerroProperty = AttachedOwner::attached_property();
    assert!(Some(expected) == value_of::<&'static FerroProperty>(&result));
}

#[test]
fn convert_from_throws_for_nonexistent_property() {
    let _base = ferro_property_converter_test();
    let target = FerroPropertyTypeConverter::new();
    let style = class1_style();
    let context = create_context(Some(style));

    let ex = target
        .convert_from(Some(&context), None, Some(&boxed("Nonexistent".to_string())))
        .expect_err("the conversion must fail with a XAML load exception");

    assert_eq!("Could not find property 'Class1.Nonexistent'.", ex.message());
}

#[test]
fn convert_from_throws_for_nonexistent_attached_property() {
    let _base = ferro_property_converter_test();
    let target = FerroPropertyTypeConverter::new();
    let style = class1_style();
    let context = create_context(Some(style));

    let ex = target
        .convert_from(Some(&context), None, Some(&boxed("AttachedOwner.NonExistent".to_string())))
        .expect_err("the conversion must fail with a XAML load exception");

    assert_eq!("Could not find property 'AttachedOwner.NonExistent'.", ex.message());
}
