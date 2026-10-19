//! `StringToSingleTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToSingleTypeConverterTests.cs`).

use super::{assert_within, converted};
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const PARSED_SINGLE: f32 = 123.456;
const NEGATIVE_SINGLE: f32 = -123.456;
const SCIENTIFIC_SINGLE: f32 = 1.23E+5;
const TYPED_SINGLE: f32 = 456.789;
const SINGLE_TOLERANCE: f32 = 0.001;
const SCIENTIFIC_TOLERANCE: f32 = 0.1;

fn convert(value: &str) -> Option<f32> {
    StringToSingleTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToSingleTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
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
fn try_convert_string_to_single_succeeds() {
    assert_within(convert("123.456").expect("converted"), PARSED_SINGLE, SINGLE_TOLERANCE);
}

#[test]
fn try_convert_null_string_returns_false() {
    assert_eq!(StringToSingleTypeConverter.try_convert(None, None), None);
}

#[test]
fn try_convert_zero_value_succeeds() {
    assert_eq!(convert("0"), Some(0.0));
}

#[test]
fn try_convert_negative_value_succeeds() {
    assert_within(convert("-123.456").expect("converted"), NEGATIVE_SINGLE, SINGLE_TOLERANCE);
}

#[test]
fn try_convert_scientific_notation_succeeds() {
    // `1.23E+5F.ToString("E2")`.
    assert_within(convert("1.23E+005").expect("converted"), SCIENTIFIC_SINGLE, SCIENTIFIC_TOLERANCE);
}

#[test]
fn try_convert_typed_valid_string_succeeds() {
    let text = String::from("456.789");
    let output = converted::<f32>(StringToSingleTypeConverter.try_convert_typed(Some(&text), None));
    assert_within(output.expect("an f32"), TYPED_SINGLE, SINGLE_TOLERANCE);
}

#[test]
fn try_convert_typed_invalid_type_returns_false() {
    assert!(StringToSingleTypeConverter.try_convert_typed(Some(&PARSED_SINGLE), None).is_none());
}

#[test]
fn try_convert_typed_null_input_returns_false() {
    assert!(StringToSingleTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn from_type_returns_string_type() {
    assert_eq!(StringToSingleTypeConverter.from_type(), ConverterType::of::<String>());
}

#[test]
fn to_type_returns_float_type() {
    assert_eq!(StringToSingleTypeConverter.to_type(), ConverterType::of::<f32>());
}
