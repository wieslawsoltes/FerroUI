//! `StringToNullableDoubleTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToNullableDoubleTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SCIENTIFIC_DOUBLE: f64 = 1.23E+10;
const PARSED_DOUBLE: f64 = 123.456;

fn convert(value: &str) -> Option<Option<f64>> {
    StringToNullableDoubleTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToNullableDoubleTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
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
fn try_convert_scientific_notation_succeeds() {
    // `1.23E+10.ToString("E2")`.
    assert_eq!(convert("1.23E+010"), Some(Some(SCIENTIFIC_DOUBLE)));
}

#[test]
fn try_convert_string_to_double_nullable_succeeds() {
    assert_eq!(convert("123.456"), Some(Some(PARSED_DOUBLE)));
}
