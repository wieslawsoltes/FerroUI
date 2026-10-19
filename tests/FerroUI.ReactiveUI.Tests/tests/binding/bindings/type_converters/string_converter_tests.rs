//! `StringConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;

#[test]
fn from_type_returns_string_type() {
    assert_eq!(StringConverter.from_type(), ConverterType::of::<String>());
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn to_type_returns_string_type() {
    assert_eq!(StringConverter.to_type(), ConverterType::of::<String>());
}

#[test]
fn try_convert_typed_empty_string_succeeds() {
    let value = String::new();
    assert_eq!(converted::<String>(StringConverter.try_convert_typed(Some(&value), None)), Some(String::new()));
}

#[test]
fn try_convert_typed_ignores_conversion_hint() {
    let value = String::from("test");
    assert_eq!(
        converted::<String>(StringConverter.try_convert_typed(Some(&value), Some(&"some hint"))),
        Some("test".to_string())
    );
}

#[test]
fn try_convert_typed_non_string_value_returns_false() {
    assert!(StringConverter.try_convert_typed(Some(&123), None).is_none());
}

#[test]
fn try_convert_typed_null_value_returns_false_and_null() {
    assert!(StringConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_string_to_string_succeeds() {
    let value = String::from("test");
    assert_eq!(converted::<String>(StringConverter.try_convert_typed(Some(&value), None)), Some("test".to_string()));
}
