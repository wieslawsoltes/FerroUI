//! `StringToTimeSpanTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToTimeSpanTypeConverterTests.cs`).

use ferroui_reactiveui::*;
use std::time::Duration;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_HOURS: f64 = 2.5;

fn text(value: &str) -> String {
    value.to_string()
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToTimeSpanTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_valid_string_succeeds() {
    // `TimeSpan.FromHours(2.5).ToString()` is `02:30:00`.
    let expected = Duration::from_secs_f64(SAMPLE_HOURS * 3_600.0);
    assert_eq!(StringToTimeSpanTypeConverter.try_convert(Some(&text("02:30:00")), None), Some(expected));
}

#[test]
fn try_convert_zero_time_span_succeeds() {
    assert_eq!(StringToTimeSpanTypeConverter.try_convert(Some(&text("00:00:00")), None), Some(Duration::ZERO));
}

#[test]
fn try_convert_null_returns_false() {
    assert_eq!(StringToTimeSpanTypeConverter.try_convert(None, None), None);
}

#[test]
fn try_convert_empty_string_returns_false() {
    assert_eq!(StringToTimeSpanTypeConverter.try_convert(Some(&String::new()), None), None);
}

#[test]
fn try_convert_invalid_string_returns_false() {
    assert_eq!(StringToTimeSpanTypeConverter.try_convert(Some(&text("invalid")), None), None);
}

/// The other forms that `TimeSpan.TryParse` reads, which upstream's tests do not reach.
#[test]
fn try_convert_other_forms() {
    let parse = |value: &str| StringToTimeSpanTypeConverter.try_convert(Some(&text(value)), None);
    assert_eq!(parse("3"), Some(Duration::from_secs(3 * 86_400)));
    assert_eq!(parse("1.02:03:04.5"), Some(Duration::new(86_400 + 2 * 3_600 + 3 * 60 + 4, 500_000_000)));
    assert_eq!(parse("1:02:03:04"), Some(Duration::from_secs(86_400 + 2 * 3_600 + 3 * 60 + 4)));
    assert_eq!(parse(" 10:30 "), Some(Duration::from_secs(10 * 3_600 + 30 * 60)));
    assert_eq!(parse("24:00:00"), None);
    assert_eq!(parse("-00:00:00"), Some(Duration::ZERO));
    assert_eq!(parse("-00:30:00"), None);
}
