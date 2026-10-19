//! `TimeSpanToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/TimeSpanToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;
use std::time::Duration;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_HOURS: f64 = 2.5;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(TimeSpanToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_time_span_succeeds() {
    // `TimeSpan.FromHours(2.5).ToString()`.
    let value = Duration::from_secs_f64(SAMPLE_HOURS * 3_600.0);
    assert_eq!(TimeSpanToStringTypeConverter.try_convert(Some(&value), None), Some("02:30:00".to_string()));
}

#[test]
fn try_convert_zero_time_span_succeeds() {
    assert_eq!(TimeSpanToStringTypeConverter.try_convert(Some(&Duration::ZERO), None), Some("00:00:00".to_string()));
}

// Not ported: TryConvert_NegativeTimeSpan_Succeeds - a `Duration` (the port of `TimeSpan`) cannot be negative

/// The days and the ticks of the constant format, which upstream's tests do not reach.
#[test]
fn try_convert_days_and_fraction() {
    let value = Duration::new(86_400 + 2 * 3_600 + 3 * 60 + 4, 500_000_000);
    assert_eq!(TimeSpanToStringTypeConverter.try_convert(Some(&value), None), Some("1.02:03:04.5000000".to_string()));
}
