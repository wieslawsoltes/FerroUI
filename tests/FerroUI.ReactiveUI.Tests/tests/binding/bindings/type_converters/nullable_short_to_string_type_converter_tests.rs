//! `NullableShortToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableShortToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const CONVERSION_HINT: i32 = 5;
const HEX_SHORT: i16 = 255;
const PADDED_SHORT: i16 = 42;
const SAMPLE_SHORT: i16 = 12_345;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableShortToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(NullableShortToStringTypeConverter.try_convert(Some(&Some(i16::MAX)), None), Some("32767".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(NullableShortToStringTypeConverter.try_convert(Some(&Some(i16::MIN)), None), Some("-32768".to_string()));
}

#[test]
fn try_convert_null_value_returns_true() {
    assert!(NullableShortToStringTypeConverter.try_convert(None, None).is_some());
}

#[test]
fn try_convert_short_nullable_to_string_succeeds() {
    assert_eq!(NullableShortToStringTypeConverter.try_convert(Some(&Some(SAMPLE_SHORT)), None), Some("12345".to_string()));
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    assert_eq!(
        NullableShortToStringTypeConverter.try_convert(Some(&Some(PADDED_SHORT)), Some(&CONVERSION_HINT)),
        Some("00042".to_string())
    );
}

#[test]
fn try_convert_with_string_format_hint_formats_correctly() {
    assert_eq!(NullableShortToStringTypeConverter.try_convert(Some(&Some(HEX_SHORT)), Some(&"X")), Some("FF".to_string()));
}
