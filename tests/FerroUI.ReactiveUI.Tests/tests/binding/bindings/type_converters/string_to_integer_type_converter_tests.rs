//! `StringToIntegerTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToIntegerTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const OUT_OF_RANGE_VALUE: f64 = 123.456;
const PARSED_INTEGER: i32 = 123_456;
const NEGATIVE_INTEGER: i32 = -123_456;
const TYPED_INTEGER: i32 = 789;

fn convert(value: &str) -> Option<i32> {
    StringToIntegerTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToIntegerTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_empty_string_returns_false() {
    assert_eq!(convert(""), None);
}

#[test]
fn try_convert_invalid_string_returns_false() {
    assert_eq!(convert("invalid"), None);
}

#[test]
fn try_convert_out_of_range_value_returns_false() {
    assert_eq!(convert("9999999999"), None);
}

#[test]
fn try_convert_string_to_int_succeeds() {
    assert_eq!(convert("123456"), Some(PARSED_INTEGER));
}

#[test]
fn try_convert_null_string_returns_false() {
    assert_eq!(StringToIntegerTypeConverter.try_convert(None, None), None);
}

#[test]
fn try_convert_zero_value_succeeds() {
    assert_eq!(convert("0"), Some(0));
}

#[test]
fn try_convert_negative_value_succeeds() {
    assert_eq!(convert("-123456"), Some(NEGATIVE_INTEGER));
}

#[test]
fn try_convert_typed_valid_string_succeeds() {
    let text = String::from("789");
    assert_eq!(converted::<i32>(StringToIntegerTypeConverter.try_convert_typed(Some(&text), None)), Some(TYPED_INTEGER));
}

#[test]
fn try_convert_typed_invalid_type_returns_false() {
    assert!(StringToIntegerTypeConverter.try_convert_typed(Some(&OUT_OF_RANGE_VALUE), None).is_none());
}

#[test]
fn try_convert_typed_null_input_returns_false() {
    assert!(StringToIntegerTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn from_type_returns_string_type() {
    assert_eq!(StringToIntegerTypeConverter.from_type(), ConverterType::of::<String>());
}

#[test]
fn to_type_returns_int_type() {
    assert_eq!(StringToIntegerTypeConverter.to_type(), ConverterType::of::<i32>());
}

/// White space and a leading sign, as `Int32.TryParse` reads them; upstream's tests do not reach
/// them.
#[test]
fn try_convert_white_space_and_sign() {
    assert_eq!(convert(" +42 "), Some(42));
    assert_eq!(convert("4 2"), None);
    assert_eq!(convert("1,000"), None);
}
