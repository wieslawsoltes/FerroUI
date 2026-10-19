//! `IntegerToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/IntegerToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const CONVERSION_HINT: i32 = 8;
const EXPECTED_AFFINITY: i32 = 2;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(IntegerToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_int_to_string_succeeds() {
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&123_456), None), Some("123456".to_string()));
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&i32::MAX), None), Some("2147483647".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&i32::MIN), None), Some("-2147483648".to_string()));
}

#[test]
fn try_convert_negative_value_succeeds() {
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&-123_456), None), Some("-123456".to_string()));
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&42), Some(&CONVERSION_HINT)), Some("00000042".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_custom_format() {
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&42), Some(&"000")), Some("042".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_hex_format() {
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&255), Some(&"X")), Some("FF".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_hex_format_lowercase() {
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&255), Some(&"x8")), Some("000000ff".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_number_format() {
    // A `String` hint reads as a `&str` one does.
    let format = String::from("N0");
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&1_234_567), Some(&format)), Some("1,234,567".to_string()));
}
