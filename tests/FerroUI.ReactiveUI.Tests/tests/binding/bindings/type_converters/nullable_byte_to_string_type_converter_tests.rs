//! `NullableByteToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableByteToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const CONVERSION_HINT: i32 = 3;
const HEX_BYTE: u8 = 255;
const PADDED_BYTE: u8 = 5;
const SAMPLE_BYTE: u8 = 123;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableByteToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_byte_nullable_to_string_succeeds() {
    assert_eq!(NullableByteToStringTypeConverter.try_convert(Some(&Some(SAMPLE_BYTE)), None), Some("123".to_string()));
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(NullableByteToStringTypeConverter.try_convert(Some(&Some(u8::MAX)), None), Some("255".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(NullableByteToStringTypeConverter.try_convert(Some(&Some(u8::MIN)), None), Some("0".to_string()));
}

#[test]
fn try_convert_null_value_returns_true() {
    // Upstream converts null to a null string; the port converts it to an empty string.
    assert_eq!(NullableByteToStringTypeConverter.try_convert(None, None), Some(String::new()));
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    assert_eq!(
        NullableByteToStringTypeConverter.try_convert(Some(&Some(PADDED_BYTE)), Some(&CONVERSION_HINT)),
        Some("005".to_string())
    );
}

#[test]
fn try_convert_with_string_format_hint_formats_correctly() {
    assert_eq!(NullableByteToStringTypeConverter.try_convert(Some(&Some(HEX_BYTE)), Some(&"X")), Some("FF".to_string()));
}
