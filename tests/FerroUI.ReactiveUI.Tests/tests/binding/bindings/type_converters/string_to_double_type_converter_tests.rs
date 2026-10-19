//! `StringToDoubleTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToDoubleTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SCIENTIFIC_DOUBLE: f64 = 1.23E+10;
const PARSED_DOUBLE: f64 = 123.456;
const NEGATIVE_DOUBLE: f64 = -123.456;
const TYPED_DOUBLE: f64 = 456.789;

fn convert(value: &str) -> Option<f64> {
    StringToDoubleTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToDoubleTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
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
fn try_convert_scientific_notation_succeeds() {
    // `1.23E+10.ToString("E2")`.
    assert_eq!(convert("1.23E+010"), Some(SCIENTIFIC_DOUBLE));
}

#[test]
fn try_convert_string_to_double_succeeds() {
    assert_eq!(convert("123.456"), Some(PARSED_DOUBLE));
}

#[test]
fn try_convert_null_string_returns_false() {
    assert_eq!(StringToDoubleTypeConverter.try_convert(None, None), None);
}

#[test]
fn try_convert_zero_value_succeeds() {
    assert_eq!(convert("0"), Some(0.0));
}

#[test]
fn try_convert_negative_value_succeeds() {
    assert_eq!(convert("-123.456"), Some(NEGATIVE_DOUBLE));
}

#[test]
fn try_convert_typed_valid_string_succeeds() {
    let text = String::from("456.789");
    assert_eq!(converted::<f64>(StringToDoubleTypeConverter.try_convert_typed(Some(&text), None)), Some(TYPED_DOUBLE));
}

#[test]
fn try_convert_typed_invalid_type_returns_false() {
    assert!(StringToDoubleTypeConverter.try_convert_typed(Some(&PARSED_DOUBLE), None).is_none());
}

#[test]
fn try_convert_typed_null_input_returns_false() {
    assert!(StringToDoubleTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn from_type_returns_string_type() {
    assert_eq!(StringToDoubleTypeConverter.from_type(), ConverterType::of::<String>());
}

#[test]
fn to_type_returns_double_type() {
    assert_eq!(StringToDoubleTypeConverter.to_type(), ConverterType::of::<f64>());
}

/// Group separators, white space and the symbols of `Double.TryParse`, which upstream's tests do
/// not reach.
#[test]
fn try_convert_number_styles() {
    assert_eq!(convert(" 1,234.5 "), Some(1_234.5));
    assert_eq!(convert(".5"), Some(0.5));
    assert_eq!(convert("Infinity"), Some(f64::INFINITY));
    assert_eq!(convert("-Infinity"), Some(f64::NEG_INFINITY));
    assert!(convert("NaN").is_some_and(f64::is_nan));
    assert_eq!(convert(",5"), None);
    assert_eq!(convert("1e"), None);
}
