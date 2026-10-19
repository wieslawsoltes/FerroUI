//! `StringToNullableLongTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToNullableLongTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const PARSED_LONG: i64 = 123_456_789_012;

fn convert(value: &str) -> Option<Option<i64>> {
    StringToNullableLongTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToNullableLongTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_empty_string_returns_true() {
    assert_eq!(convert(""), Some(None));
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
fn try_convert_string_to_long_nullable_succeeds() {
    assert_eq!(convert("123456789012"), Some(Some(PARSED_LONG)));
}
