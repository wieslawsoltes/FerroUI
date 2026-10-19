//! `StringToNullableByteTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToNullableByteTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const PARSED_BYTE: u8 = 123;

fn convert(value: &str) -> Option<Option<u8>> {
    StringToNullableByteTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToNullableByteTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
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
fn try_convert_negative_value_returns_false() {
    assert_eq!(convert("-1"), None);
}

#[test]
fn try_convert_out_of_range_value_returns_false() {
    assert_eq!(convert("999"), None);
}

#[test]
fn try_convert_string_to_byte_nullable_succeeds() {
    assert_eq!(convert("123"), Some(Some(PARSED_BYTE)));
}
