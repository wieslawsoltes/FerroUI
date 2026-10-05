//! Port of the upstream `DefaultValueConverterTests`.
//!
//! Conversions that the managed runtime finds through reflection (enum
//! names and values, type converters, explicit cast operators) are declared
//! to the value type table by the type that owns them; the tests declare
//! them for their own types the same way an application does.
//!
//! Not ported: `Can_Convert_From_Delegate_To_Command` and
//! `Can_Convert_From_Delegate_To_Command_No_Parameters` (delegates are not
//! binding values: methods bind as commands through the method nodes).
//! `Can_Convert_Decimal_To_NullableDouble` converts a 64-bit integer, as
//! there is no decimal type.

use super::*;
use crate::animation::TimeSpan;
use crate::data::converters::DefaultValueConverter;
use crate::data::core::{ValueType, ValueTypes};
use crate::data::BindingNotification;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestEnum {
    Foo,
    Bar,
}

impl fmt::Display for TestEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// The equivalent of the upstream orientation enum: a second enum type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq)]
struct ExplicitDouble {
    value: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomType {
    value: i32,
}

fn register_conversions() {
    ValueTypes::register_display::<TestEnum>();
    ValueTypes::register_parse::<TestEnum>(|s| match s {
        "Foo" => Some(TestEnum::Foo),
        "Bar" => Some(TestEnum::Bar),
        _ => None,
    });
    ValueTypes::register_conversion::<i32, TestEnum>(|v| match v {
        0 => Some(TestEnum::Foo),
        1 => Some(TestEnum::Bar),
        _ => None,
    });
    ValueTypes::register_conversion::<TestEnum, i32>(|v| Some(*v as i32));
    ValueTypes::register_conversion::<i32, Orientation>(|v| match v {
        0 => Some(Orientation::Horizontal),
        1 => Some(Orientation::Vertical),
        _ => None,
    });
    ValueTypes::register_conversion::<ExplicitDouble, f64>(|v| Some(v.value));
    ValueTypes::register_conversion::<CustomType, i32>(|v| Some(v.value));
    ValueTypes::register_conversion::<i32, CustomType>(|v| Some(CustomType { value: *v }));
    ValueTypes::register_parse::<TimeSpan>(|s| s.parse().ok());
}

fn convert<T: PartialEq + 'static, TTo: 'static>(value: T) -> Option<BoxedValue> {
    register_conversions();
    DefaultValueConverter::instance()
        .convert(Some(&boxed(value)), ValueType::of::<TTo>(), None)
        .expect("the default converter does not fail")
}

#[track_caller]
fn assert_value<T: PartialEq + fmt::Debug + 'static>(result: Option<BoxedValue>, expected: T) {
    let result = result.expect("a value");
    assert_eq!(result.downcast_ref::<T>(), Some(&expected), "the result is a {}", result.type_name());
}

#[test]
fn can_convert_string_to_int() {
    let result = convert::<_, i32>(s("5"));

    assert_value(result, 5);
}

#[test]
fn can_convert_string_to_double() {
    let result = convert::<_, f64>(s("5"));

    assert_value(result, 5.0);
}

#[test]
fn do_not_throw_on_invalid_input_for_nullable_int() {
    let result = convert::<_, Option<i32>>(s("<not-a-number>"));

    assert!(result.expect("a value").is::<BindingNotification>());
}

#[test]
fn can_convert_decimal_to_nullable_double() {
    let result = convert::<_, Option<f64>>(5i64);

    assert_value(result, Some(5.0));
}

#[test]
fn can_convert_custom_type_to_int() {
    let result = convert::<_, i32>(CustomType { value: 123 });

    assert_value(result, 123);
}

#[test]
fn can_convert_int_to_custom_type() {
    let result = convert::<_, CustomType>(123);

    assert_value(result, CustomType { value: 123 });
}

#[test]
fn can_convert_string_to_enum() {
    let result = convert::<_, TestEnum>(s("Bar"));

    assert_value(result, TestEnum::Bar);
}

#[test]
fn can_convert_string_to_time_span() {
    let result = convert::<_, TimeSpan>(s("00:00:10"));

    assert_value(result, TimeSpan::from_seconds(10.0));
}

#[test]
fn can_convert_int_to_enum() {
    let result = convert::<_, TestEnum>(1);

    assert_value(result, TestEnum::Bar);
}

#[test]
fn can_convert_double_to_string() {
    let result = convert::<_, String>(5.0);

    assert_value(result, s("5"));
}

#[test]
fn can_convert_enum_to_int() {
    let result = convert::<_, i32>(TestEnum::Bar);

    assert_value(result, 1);
}

#[test]
fn can_convert_enum_to_string() {
    let result = convert::<_, String>(TestEnum::Bar);

    assert_value(result, s("Bar"));
}

#[test]
fn can_use_explicit_cast() {
    let result = convert::<_, f64>(ExplicitDouble { value: 5.0 });

    assert_value(result, 5.0);
}

#[test]
fn cannot_convert_between_different_enum_types() {
    let result = convert::<_, Orientation>(TestEnum::Foo);

    assert!(result.expect("a value").is::<BindingNotification>());
}
