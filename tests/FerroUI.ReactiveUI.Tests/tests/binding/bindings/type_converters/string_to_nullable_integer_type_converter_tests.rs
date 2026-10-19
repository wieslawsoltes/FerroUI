//! `StringToNullableIntegerTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToNullableIntegerTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const PARSED_INTEGER: i32 = 123_456;

fn convert(value: &str) -> Option<Option<i32>> {
    StringToNullableIntegerTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToNullableIntegerTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
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
    assert_eq!(convert("9999999999"), None);
}

#[test]
fn try_convert_string_to_int_nullable_succeeds() {
    assert_eq!(convert("123456"), Some(Some(PARSED_INTEGER)));
}

/// No text converts to `None` as well, which upstream's file does not test.
#[test]
fn try_convert_null_returns_null() {
    assert_eq!(StringToNullableIntegerTypeConverter.try_convert(None, None), Some(None));
}
