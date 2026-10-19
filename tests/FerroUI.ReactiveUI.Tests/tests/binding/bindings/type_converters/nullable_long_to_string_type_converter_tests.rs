//! `NullableLongToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableLongToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const CONVERSION_HINT: i32 = 10;
const HEX_LONG: i64 = 255;
const PADDED_LONG: i64 = 42;
const SAMPLE_LONG: i64 = 123_456_789_012;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableLongToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_long_nullable_to_string_succeeds() {
    assert_eq!(NullableLongToStringTypeConverter.try_convert(Some(&Some(SAMPLE_LONG)), None), Some("123456789012".to_string()));
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(
        NullableLongToStringTypeConverter.try_convert(Some(&Some(i64::MAX)), None),
        Some("9223372036854775807".to_string())
    );
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(
        NullableLongToStringTypeConverter.try_convert(Some(&Some(i64::MIN)), None),
        Some("-9223372036854775808".to_string())
    );
}

#[test]
fn try_convert_null_value_returns_true() {
    assert!(NullableLongToStringTypeConverter.try_convert(None, None).is_some());
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    assert_eq!(
        NullableLongToStringTypeConverter.try_convert(Some(&Some(PADDED_LONG)), Some(&CONVERSION_HINT)),
        Some("0000000042".to_string())
    );
}

#[test]
fn try_convert_with_string_format_hint_formats_correctly() {
    assert_eq!(NullableLongToStringTypeConverter.try_convert(Some(&Some(HEX_LONG)), Some(&"X")), Some("FF".to_string()));
}
