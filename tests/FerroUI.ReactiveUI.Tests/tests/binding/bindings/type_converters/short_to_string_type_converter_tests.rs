//! `ShortToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/ShortToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const CONVERSION_HINT: i32 = 5;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(ShortToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(ShortToStringTypeConverter.try_convert(Some(&i16::MAX), None), Some("32767".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(ShortToStringTypeConverter.try_convert(Some(&i16::MIN), None), Some("-32768".to_string()));
}

#[test]
fn try_convert_short_to_string_succeeds() {
    assert_eq!(ShortToStringTypeConverter.try_convert(Some(&12_345), None), Some("12345".to_string()));
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    assert_eq!(ShortToStringTypeConverter.try_convert(Some(&42), Some(&CONVERSION_HINT)), Some("00042".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_formats_correctly() {
    assert_eq!(ShortToStringTypeConverter.try_convert(Some(&255), Some(&"X")), Some("FF".to_string()));
}
