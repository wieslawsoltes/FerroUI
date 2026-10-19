//! `StringToByteTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToByteTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const PARSED_BYTE: u8 = 123;
const MAX_BYTE: u8 = 255;
const TYPED_BYTE: u8 = 100;

fn convert(value: &str) -> Option<u8> {
    StringToByteTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToByteTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_empty_string_returns_false() {
    assert_eq!(convert(""), None);
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
fn try_convert_string_to_byte_succeeds() {
    assert_eq!(convert("123"), Some(PARSED_BYTE));
}

#[test]
fn try_convert_null_string_returns_false() {
    assert_eq!(StringToByteTypeConverter.try_convert(None, None), None);
}

#[test]
fn try_convert_zero_value_succeeds() {
    assert_eq!(convert("0"), Some(0));
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(convert("255"), Some(MAX_BYTE));
}

#[test]
fn try_convert_typed_valid_string_succeeds() {
    let text = String::from("100");
    assert_eq!(converted::<u8>(StringToByteTypeConverter.try_convert_typed(Some(&text), None)), Some(TYPED_BYTE));
}

#[test]
fn try_convert_typed_invalid_type_returns_false() {
    assert!(StringToByteTypeConverter.try_convert_typed(Some(&PARSED_BYTE), None).is_none());
}

#[test]
fn try_convert_typed_null_input_returns_false() {
    assert!(StringToByteTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn from_type_returns_string_type() {
    assert_eq!(StringToByteTypeConverter.from_type(), ConverterType::of::<String>());
}

#[test]
fn to_type_returns_byte_type() {
    assert_eq!(StringToByteTypeConverter.to_type(), ConverterType::of::<u8>());
}
