//! `ByteToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/ByteToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const CONVERSION_HINT: i32 = 3;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(ByteToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_byte_to_string_succeeds() {
    assert_eq!(ByteToStringTypeConverter.try_convert(Some(&123), None), Some("123".to_string()));
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(ByteToStringTypeConverter.try_convert(Some(&u8::MAX), None), Some("255".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(ByteToStringTypeConverter.try_convert(Some(&u8::MIN), None), Some("0".to_string()));
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    assert_eq!(ByteToStringTypeConverter.try_convert(Some(&5), Some(&CONVERSION_HINT)), Some("005".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_formats_correctly() {
    assert_eq!(ByteToStringTypeConverter.try_convert(Some(&255), Some(&"X")), Some("FF".to_string()));
}
