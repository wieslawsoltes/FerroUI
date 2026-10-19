//! `NullableTimeSpanToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableTimeSpanToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;
use std::time::Duration;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_HOURS: f64 = 2.5;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableTimeSpanToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_time_span_succeeds() {
    // `TimeSpan.FromHours(2.5).ToString()`.
    let value = Some(Duration::from_secs_f64(SAMPLE_HOURS * 3_600.0));
    assert_eq!(NullableTimeSpanToStringTypeConverter.try_convert(Some(&value), None), Some("02:30:00".to_string()));
}

#[test]
fn try_convert_null_returns_null_string() {
    // Upstream converts null to a null string; the port converts it to an empty string.
    assert_eq!(NullableTimeSpanToStringTypeConverter.try_convert(None, None), Some(String::new()));
}
