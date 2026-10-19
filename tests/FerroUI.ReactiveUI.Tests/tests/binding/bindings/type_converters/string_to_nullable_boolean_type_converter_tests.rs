//! `StringToNullableBooleanTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToNullableBooleanTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;

fn convert(value: &str) -> Option<Option<bool>> {
    StringToNullableBooleanTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToNullableBooleanTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_true_string_succeeds() {
    assert_eq!(convert("True"), Some(Some(true)));
}

#[test]
fn try_convert_false_string_succeeds() {
    assert_eq!(convert("False"), Some(Some(false)));
}

#[test]
fn try_convert_true_lowercase_succeeds() {
    assert_eq!(convert("true"), Some(Some(true)));
}

#[test]
fn try_convert_false_lowercase_succeeds() {
    assert_eq!(convert("false"), Some(Some(false)));
}

#[test]
fn try_convert_null_returns_null() {
    assert_eq!(StringToNullableBooleanTypeConverter.try_convert(None, None), Some(None));
}

#[test]
fn try_convert_empty_string_returns_null() {
    assert_eq!(convert(""), Some(None));
}

#[test]
fn try_convert_invalid_string_returns_false() {
    assert_eq!(convert("invalid"), None);
}
