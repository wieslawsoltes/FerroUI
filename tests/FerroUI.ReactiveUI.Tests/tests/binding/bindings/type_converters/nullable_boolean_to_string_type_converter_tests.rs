//! `NullableBooleanToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableBooleanToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableBooleanToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_true_returns_true() {
    assert_eq!(NullableBooleanToStringTypeConverter.try_convert(Some(&Some(true)), None), Some("True".to_string()));
}

#[test]
fn try_convert_false_returns_false() {
    assert_eq!(NullableBooleanToStringTypeConverter.try_convert(Some(&Some(false)), None), Some("False".to_string()));
}

#[test]
fn try_convert_null_returns_null_string() {
    // Upstream converts null to a null string; the port converts it to an empty string.
    assert_eq!(NullableBooleanToStringTypeConverter.try_convert(None, None), Some(String::new()));
    assert_eq!(NullableBooleanToStringTypeConverter.try_convert(Some(&None), None), Some(String::new()));
}
