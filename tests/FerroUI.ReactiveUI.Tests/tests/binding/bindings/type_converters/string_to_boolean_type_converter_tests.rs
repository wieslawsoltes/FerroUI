//! `StringToBooleanTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToBooleanTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;

fn convert(value: &str) -> Option<bool> {
    StringToBooleanTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToBooleanTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_true_string_succeeds() {
    assert_eq!(convert("True"), Some(true));
}

#[test]
fn try_convert_false_string_succeeds() {
    assert_eq!(convert("False"), Some(false));
}

#[test]
fn try_convert_true_lowercase_succeeds() {
    assert_eq!(convert("true"), Some(true));
}

#[test]
fn try_convert_false_lowercase_succeeds() {
    assert_eq!(convert("false"), Some(false));
}

#[test]
fn try_convert_null_returns_false() {
    assert_eq!(StringToBooleanTypeConverter.try_convert(None, None), None);
}

#[test]
fn try_convert_empty_string_returns_false() {
    assert_eq!(convert(""), None);
}

#[test]
fn try_convert_invalid_string_returns_false() {
    assert_eq!(convert("invalid"), None);
}
