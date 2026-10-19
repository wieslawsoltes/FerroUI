//! `NullableSingleToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableSingleToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const FORMATTED_SINGLE: f32 = 42.5;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableSingleToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(NullableSingleToStringTypeConverter.try_convert(Some(&Some(f32::MAX)), None), Some("3.4028235E+38".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(NullableSingleToStringTypeConverter.try_convert(Some(&Some(f32::MIN)), None), Some("-3.4028235E+38".to_string()));
}

#[test]
fn try_convert_negative_value_succeeds() {
    assert_eq!(NullableSingleToStringTypeConverter.try_convert(Some(&Some(-123.456)), None), Some("-123.456".to_string()));
}

#[test]
fn try_convert_null_value_returns_true() {
    assert!(NullableSingleToStringTypeConverter.try_convert(None, None).is_some());
}

#[test]
fn try_convert_single_nullable_to_string_succeeds() {
    assert_eq!(NullableSingleToStringTypeConverter.try_convert(Some(&Some(123.456)), None), Some("123.456".to_string()));
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    // Upstream passes its constant `ExpectedAffinity` (2) as the number of decimals.
    assert_eq!(
        NullableSingleToStringTypeConverter.try_convert(Some(&Some(FORMATTED_SINGLE)), Some(&EXPECTED_AFFINITY)),
        Some("42.50".to_string())
    );
}

#[test]
fn try_convert_with_string_format_hint_formats_correctly() {
    assert_eq!(NullableSingleToStringTypeConverter.try_convert(Some(&Some(42.5)), Some(&"E2")), Some("4.25E+001".to_string()));
}
