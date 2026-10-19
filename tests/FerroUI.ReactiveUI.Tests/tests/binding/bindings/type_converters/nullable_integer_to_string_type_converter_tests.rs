//! `NullableIntegerToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableIntegerToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const CONVERSION_HINT: i32 = 8;
const HEX_INTEGER: i32 = 255;
const PADDED_INTEGER: i32 = 42;
const SAMPLE_INTEGER: i32 = 123_456;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableIntegerToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_int_nullable_to_string_succeeds() {
    assert_eq!(NullableIntegerToStringTypeConverter.try_convert(Some(&Some(SAMPLE_INTEGER)), None), Some("123456".to_string()));
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(NullableIntegerToStringTypeConverter.try_convert(Some(&Some(i32::MAX)), None), Some("2147483647".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(NullableIntegerToStringTypeConverter.try_convert(Some(&Some(i32::MIN)), None), Some("-2147483648".to_string()));
}

#[test]
fn try_convert_null_value_returns_true() {
    assert!(NullableIntegerToStringTypeConverter.try_convert(None, None).is_some());
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    assert_eq!(
        NullableIntegerToStringTypeConverter.try_convert(Some(&Some(PADDED_INTEGER)), Some(&CONVERSION_HINT)),
        Some("00000042".to_string())
    );
}

#[test]
fn try_convert_with_string_format_hint_formats_correctly() {
    assert_eq!(NullableIntegerToStringTypeConverter.try_convert(Some(&Some(HEX_INTEGER)), Some(&"X")), Some("FF".to_string()));
}
