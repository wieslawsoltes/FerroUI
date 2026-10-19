//! `LongToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/LongToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const CONVERSION_HINT: i32 = 10;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(LongToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_long_to_string_succeeds() {
    assert_eq!(LongToStringTypeConverter.try_convert(Some(&123_456_789_012), None), Some("123456789012".to_string()));
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(LongToStringTypeConverter.try_convert(Some(&i64::MAX), None), Some("9223372036854775807".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(LongToStringTypeConverter.try_convert(Some(&i64::MIN), None), Some("-9223372036854775808".to_string()));
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    assert_eq!(LongToStringTypeConverter.try_convert(Some(&42), Some(&CONVERSION_HINT)), Some("0000000042".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_formats_correctly() {
    assert_eq!(LongToStringTypeConverter.try_convert(Some(&255), Some(&"X")), Some("FF".to_string()));
}
