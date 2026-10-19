//! `DoubleToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/DoubleToStringTypeConverterTests.cs`).
//!
//! The expected texts are those of .NET with the en-US culture of upstream's tests, which match
//! the invariant culture of the port for these formats.

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(DoubleToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_double_to_string_succeeds() {
    assert_eq!(DoubleToStringTypeConverter.try_convert(Some(&123.456), None), Some("123.456".to_string()));
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(DoubleToStringTypeConverter.try_convert(Some(&f64::MAX), None), Some("1.7976931348623157E+308".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(DoubleToStringTypeConverter.try_convert(Some(&f64::MIN), None), Some("-1.7976931348623157E+308".to_string()));
}

#[test]
fn try_convert_negative_value_succeeds() {
    assert_eq!(DoubleToStringTypeConverter.try_convert(Some(&-123.456), None), Some("-123.456".to_string()));
}

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    // Upstream passes its constant `ExpectedAffinity` (2) as the number of decimals.
    assert_eq!(DoubleToStringTypeConverter.try_convert(Some(&42.5), Some(&EXPECTED_AFFINITY)), Some("42.50".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_custom_precision() {
    assert_eq!(DoubleToStringTypeConverter.try_convert(Some(&0.123456789), Some(&"0.0000")), Some("0.1235".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_general_format() {
    assert_eq!(DoubleToStringTypeConverter.try_convert(Some(&123.456), Some(&"G")), Some("123.456".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_round_trip_format() {
    assert_eq!(
        DoubleToStringTypeConverter.try_convert(Some(&123.456789012345), Some(&"R")),
        Some("123.456789012345".to_string())
    );
}

#[test]
fn try_convert_with_string_format_hint_scientific_format() {
    assert_eq!(DoubleToStringTypeConverter.try_convert(Some(&12345.6789), Some(&"E3")), Some("1.235E+004".to_string()));
}
