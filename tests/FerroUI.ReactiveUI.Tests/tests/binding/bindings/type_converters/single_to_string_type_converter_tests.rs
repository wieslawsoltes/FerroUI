//! `SingleToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/SingleToStringTypeConverterTests.cs`).
//!
//! The expected texts are those of .NET with the en-US culture of upstream's tests, which match
//! the invariant culture of the port for these formats.

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(SingleToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_max_value_succeeds() {
    assert_eq!(SingleToStringTypeConverter.try_convert(Some(&f32::MAX), None), Some("3.4028235E+38".to_string()));
}

#[test]
fn try_convert_min_value_succeeds() {
    assert_eq!(SingleToStringTypeConverter.try_convert(Some(&f32::MIN), None), Some("-3.4028235E+38".to_string()));
}

#[test]
fn try_convert_negative_value_succeeds() {
    assert_eq!(SingleToStringTypeConverter.try_convert(Some(&-123.456), None), Some("-123.456".to_string()));
}

#[test]
fn try_convert_single_to_string_succeeds() {
    assert_eq!(SingleToStringTypeConverter.try_convert(Some(&123.456), None), Some("123.456".to_string()));
}

// Not ported: TryConvert_CommaDecimalCulture_UsesCurrentCulture - culture (the port formats with the invariant culture)

#[test]
fn try_convert_with_conversion_hint_formats_correctly() {
    // Upstream passes its constant `ExpectedAffinity` (2) as the number of decimals.
    assert_eq!(SingleToStringTypeConverter.try_convert(Some(&42.5), Some(&EXPECTED_AFFINITY)), Some("42.50".to_string()));
}

#[test]
fn try_convert_with_string_format_hint_formats_correctly() {
    assert_eq!(SingleToStringTypeConverter.try_convert(Some(&42.5), Some(&"E2")), Some("4.25E+001".to_string()));
}
