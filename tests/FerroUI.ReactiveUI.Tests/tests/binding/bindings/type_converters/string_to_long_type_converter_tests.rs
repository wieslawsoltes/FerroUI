//! `StringToLongTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToLongTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_VALUE: i32 = 123_456;
const PARSED_LONG: i64 = 123_456_789_012;
const NEGATIVE_LONG: i64 = -123_456_789_012;
const TYPED_LONG: i64 = 987_654_321_098;

fn convert(value: &str) -> Option<i64> {
    StringToLongTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToLongTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
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
    assert_eq!(convert("99999999999999999999"), None);
}

#[test]
fn try_convert_string_to_long_succeeds() {
    assert_eq!(convert("123456789012"), Some(PARSED_LONG));
}

#[test]
fn try_convert_null_string_returns_false() {
    assert_eq!(StringToLongTypeConverter.try_convert(None, None), None);
}

#[test]
fn try_convert_zero_value_succeeds() {
    assert_eq!(convert("0"), Some(0));
}

#[test]
fn try_convert_negative_value_succeeds() {
    assert_eq!(convert("-123456789012"), Some(NEGATIVE_LONG));
}

#[test]
fn try_convert_typed_valid_string_succeeds() {
    let text = String::from("987654321098");
    assert_eq!(converted::<i64>(StringToLongTypeConverter.try_convert_typed(Some(&text), None)), Some(TYPED_LONG));
}

#[test]
fn try_convert_typed_invalid_type_returns_false() {
    assert!(StringToLongTypeConverter.try_convert_typed(Some(&SAMPLE_VALUE), None).is_none());
}

#[test]
fn try_convert_typed_null_input_returns_false() {
    assert!(StringToLongTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn from_type_returns_string_type() {
    assert_eq!(StringToLongTypeConverter.from_type(), ConverterType::of::<String>());
}

#[test]
fn to_type_returns_long_type() {
    assert_eq!(StringToLongTypeConverter.to_type(), ConverterType::of::<i64>());
}
