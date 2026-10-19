//! `StringToNullableShortTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToNullableShortTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const PARSED_SHORT: i16 = 12_345;

fn convert(value: &str) -> Option<Option<i16>> {
    StringToNullableShortTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToNullableShortTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
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
    assert_eq!(convert("99999"), None);
}

#[test]
fn try_convert_string_to_short_nullable_succeeds() {
    assert_eq!(convert("12345"), Some(Some(PARSED_SHORT)));
}
