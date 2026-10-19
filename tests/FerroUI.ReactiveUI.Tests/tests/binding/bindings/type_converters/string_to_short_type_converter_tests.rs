//! `StringToShortTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToShortTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_VALUE: i32 = 1_234;
const PARSED_SHORT: i16 = 12_345;
const NEGATIVE_SHORT: i16 = -12_345;
const TYPED_SHORT: i16 = 1_000;

fn convert(value: &str) -> Option<i16> {
    StringToShortTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToShortTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
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
fn try_convert_out_of_range_value_returns_false() {
    assert_eq!(convert("99999"), None);
}

#[test]
fn try_convert_string_to_short_succeeds() {
    assert_eq!(convert("12345"), Some(PARSED_SHORT));
}

#[test]
fn try_convert_null_string_returns_false() {
    assert_eq!(StringToShortTypeConverter.try_convert(None, None), None);
}

#[test]
fn try_convert_zero_value_succeeds() {
    assert_eq!(convert("0"), Some(0));
}

#[test]
fn try_convert_negative_value_succeeds() {
    assert_eq!(convert("-12345"), Some(NEGATIVE_SHORT));
}

#[test]
fn try_convert_typed_valid_string_succeeds() {
    let text = String::from("1000");
    assert_eq!(converted::<i16>(StringToShortTypeConverter.try_convert_typed(Some(&text), None)), Some(TYPED_SHORT));
}

#[test]
fn try_convert_typed_invalid_type_returns_false() {
    assert!(StringToShortTypeConverter.try_convert_typed(Some(&SAMPLE_VALUE), None).is_none());
}

#[test]
fn try_convert_typed_null_input_returns_false() {
    assert!(StringToShortTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn from_type_returns_string_type() {
    assert_eq!(StringToShortTypeConverter.from_type(), ConverterType::of::<String>());
}

#[test]
fn to_type_returns_short_type() {
    assert_eq!(StringToShortTypeConverter.to_type(), ConverterType::of::<i16>());
}
