//! `StringToNullableTimeSpanTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToNullableTimeSpanTypeConverterTests.cs`).

use ferroui_reactiveui::*;
use std::time::Duration;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_HOURS: f64 = 2.5;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToNullableTimeSpanTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_valid_string_succeeds() {
    let expected = Duration::from_secs_f64(SAMPLE_HOURS * 3_600.0);
    let text = String::from("02:30:00");
    assert_eq!(StringToNullableTimeSpanTypeConverter.try_convert(Some(&text), None), Some(Some(expected)));
}

#[test]
fn try_convert_null_returns_null() {
    assert_eq!(StringToNullableTimeSpanTypeConverter.try_convert(None, None), Some(None));
}

#[test]
fn try_convert_empty_string_returns_null() {
    assert_eq!(StringToNullableTimeSpanTypeConverter.try_convert(Some(&String::new()), None), Some(None));
}

#[test]
fn try_convert_invalid_string_returns_false() {
    let text = String::from("invalid");
    assert_eq!(StringToNullableTimeSpanTypeConverter.try_convert(Some(&text), None), None);
}
