//! The labelers of the pinned `Labelers`, the label function of an axis (`Labeler`), and the
//! date and time conversions of `LiveCharts.AsDate` and `AsTimeSpan` that the `DateTimeAxis` and
//! `TimeSpanAxis` of the source label their ticks with.

use ferroui_base::animation::SignedDuration;
use ferroui_base::utilities::{CultureInfo, DateTime, DateTimeKind, NumberFormatInfo};
use std::fmt;
use std::rc::Rc;

/// The label of a value (the source's `Func<double, string>`), shared by the snapshots of an axis.
/// Two labelers are equal when they are the same function.
#[derive(Clone)]
pub struct Labeler(Rc<dyn Fn(f64) -> String>);

impl Labeler {
    pub fn new(labeler: impl Fn(f64) -> String + 'static) -> Self {
        Self(Rc::new(labeler))
    }
    /// The label of a value.
    pub fn label(&self, value: f64) -> String {
        (self.0)(value)
    }
    /// The labeler of a `DateTimeAxis`: the value as the date of its ticks (`AsDate`), formatted
    /// by `formatter`.
    pub fn date_time(formatter: impl Fn(DateTime) -> String + 'static) -> Self {
        Self::new(move |value| formatter(as_date(value)))
    }
    /// A date labeler with a .NET date and time format string, in the conventions of the current
    /// culture (`date.ToString(format)`); an invalid format labels nothing.
    pub fn date_time_format(format: impl Into<String>) -> Self {
        let format = format.into();
        Self::date_time(move |date| {
            date.try_to_string_format(&format, &CultureInfo::current_formatting())
                .unwrap_or_default()
        })
    }
    /// The labeler of a `TimeSpanAxis`: the value as the interval of its ticks (`AsTimeSpan`),
    /// formatted by `formatter`.
    pub fn time_span(formatter: impl Fn(SignedDuration) -> String + 'static) -> Self {
        Self::new(move |value| formatter(as_time_span(value)))
    }
    /// An interval labeler with a .NET `TimeSpan` format string (`span.ToString(format)`); an
    /// invalid format labels nothing.
    pub fn time_span_format(format: impl Into<String>) -> Self {
        let format = format.into();
        Self::time_span(move |span| span.try_to_string_format(&format).unwrap_or_default())
    }
}

impl PartialEq for Labeler {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for Labeler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Labeler")
    }
}

/// The date of a number of ticks (`LiveCharts.AsDate`): a negative value is the first date, and
/// a value past the last date is the last date, where the source would overflow.
pub fn as_date(ticks: f64) -> DateTime {
    let ticks = if ticks.is_nan() || ticks < 0.0 {
        0
    } else {
        ticks as i64
    };
    DateTime::try_from_ticks(ticks, DateTimeKind::Unspecified).unwrap_or(DateTime::MAX_VALUE)
}

/// The interval of a number of ticks (`LiveCharts.AsTimeSpan`): a negative value is zero.
pub fn as_time_span(ticks: f64) -> SignedDuration {
    SignedDuration::from_ticks(if ticks.is_nan() || ticks < 0.0 {
        0
    } else {
        ticks as i64
    })
}

/// The labelers of the source (`Labelers`).
pub struct Labelers;

impl Labelers {
    /// `SixRepresentativeDigits` (the source's default labeler, `Log10_6`): up to seven decimals,
    /// a value of a million or more in millions (`M`), a value of a millionth or less in
    /// millionths (`µ`).
    pub fn six_representative_digits(value: f64) -> String {
        let l = log10_truncated(value);
        if l >= 6 {
            return format!("{} M", decimals(value / 10f64.powi(6), 7));
        }
        if l <= -6 {
            return format!("{} µ", decimals(value * 10f64.powi(6), 7));
        }
        decimals(value, 7)
    }

    /// `FormatCurrency`: the value in quadrillions (`Q`), trillions (`T`), billions (`B`) or
    /// millions (`M`) from a million on, with the symbol, digit groups of three and up to two
    /// decimals (`{symbol}#,###,##0.## {unit}`).
    pub fn format_currency(value: f64, thousands: &str, decimals: &str, symbol: &str) -> String {
        let l = log10_truncated(value);
        let (value, unit) = match l {
            l if l >= 15 => (value / 10f64.powi(15), "Q"),
            l if l >= 12 => (value / 10f64.powi(12), "T"),
            l if l >= 9 => (value / 10f64.powi(9), "B"),
            l if l >= 6 => (value / 10f64.powi(6), "M"),
            _ => (value, ""),
        };
        let rounded = format!("{:.2}", value.abs());
        let (integer, fraction) = rounded.split_once('.').unwrap_or((rounded.as_str(), ""));
        let fraction = fraction.trim_end_matches('0');
        let mut grouped = String::new();
        for (i, digit) in integer.chars().enumerate() {
            if i > 0 && (integer.len() - i) % 3 == 0 {
                grouped.push_str(thousands);
            }
            grouped.push(digit);
        }
        let negative = value < 0.0 && (integer != "0" || !fraction.is_empty());
        format!(
            "{}{symbol}{grouped}{}{fraction} {unit}",
            if negative { "-" } else { "" },
            if fraction.is_empty() { "" } else { decimals },
        )
    }

    /// `Currency`: [`format_currency`](Self::format_currency) with `,`, `.` and the currency symbol
    /// of the current culture.
    pub fn currency(value: f64) -> String {
        let symbol = NumberFormatInfo::current_info().currency_symbol().to_string();
        Self::format_currency(value, ",", ".", &symbol)
    }

    /// `BuildNamedLabeler`: the label at the index of a value (its integer part); an index out of
    /// range has an empty label.
    pub fn build_named_labeler(labels: Vec<String>) -> Labeler {
        Labeler::new(move |value| {
            let index = value.trunc();
            if !(index >= 0.0) || index >= labels.len() as f64 {
                return String::new();
            }
            labels[index as usize].clone()
        })
    }
}

/// `(int)Math.Log10(Math.Abs(value))`, zero for zero.
fn log10_truncated(value: f64) -> i32 {
    if value == 0.0 || !value.is_finite() {
        0
    } else {
        value.abs().log10() as i32
    }
}

/// The value with up to `digits` decimals and without trailing zeros (`######0.#######`).
fn decimals(value: f64, digits: usize) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let mut text = format!("{value:.digits$}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "-0" {
        text = "0".into();
    }
    text
}

#[cfg(test)]
mod tests {
    //! Ports of `CoreTests/CoreObjectsTests/LabelerTesting.cs` and
    //! `CoreTests/OtherTests/TimeSpanAxisTests.cs` at the pinned source.
    use super::*;
    use crate::Axis;

    #[test]
    fn named_labeler_null_entry_returns_empty() {
        // The source's null entry is an empty label (#1826).
        let labeler =
            Labelers::build_named_labeler(vec!["A".into(), String::new(), "C".into()]);
        assert_eq!(labeler.label(0.0), "A");
        assert_eq!(labeler.label(1.0), "");
        assert_eq!(labeler.label(2.0), "C");
        assert_eq!(labeler.label(3.0), "");
        assert_eq!(labeler.label(-1.0), "");
    }

    #[test]
    fn million() {
        let cases = [
            (100000000.0, "100 M"),
            (10000000.0, "10 M"),
            (1000000.0, "1 M"),
            (100000.0, "100000"),
            (-1000000.0, "-1 M"),
            (-10000000.0, "-10 M"),
            (-100000000.0, "-100 M"),
        ];
        for (value, expected) in cases {
            assert_eq!(Labelers::six_representative_digits(value), expected);
        }
    }

    #[test]
    fn micra() {
        let cases = [
            (0.00000001, "0.01 µ"),
            (0.0000001, "0.1 µ"),
            (0.000001, "1 µ"),
            (0.00001, "0.00001"),
            (-0.000001, "-1 µ"),
            (-0.00000001, "-0.01 µ"),
            (-0.0000001, "-0.1 µ"),
        ];
        for (value, expected) in cases {
            assert_eq!(Labelers::six_representative_digits(value), expected);
        }
    }

    #[test]
    fn currency() {
        let cases = [
            (-1000000000000000.0, "-$1 Q"),
            (-100000000000000.0, "-$100 T"),
            (-10000000000000.0, "-$10 T"),
            (-1000000000000.0, "-$1 T"),
            (-100000000000.0, "-$100 B"),
            (-10000000000.0, "-$10 B"),
            (-1000000000.0, "-$1 B"),
            (-100000000.0, "-$100 M"),
            (-10000000.0, "-$10 M"),
            (-1000000.0, "-$1 M"),
            (1000000.0, "$1 M"),
            (10000000.0, "$10 M"),
            (100000000.0, "$100 M"),
            (1000000000.0, "$1 B"),
            (10000000000.0, "$10 B"),
            (100000000000.0, "$100 B"),
            (1000000000000.0, "$1 T"),
            (10000000000000.0, "$10 T"),
            (100000000000000.0, "$100 T"),
            (1000000000000000.0, "$1 Q"),
        ];
        // The source passes the symbol of its en-US culture.
        for (value, expected) in cases {
            assert_eq!(Labelers::format_currency(value, ",", ".", "$"), expected);
        }
        assert_eq!(Labelers::format_currency(1234.5, ",", ".", "$"), "$1,234.5 ");
    }

    #[test]
    fn time_span_axis_constructor_sets_unit_width_and_min_step_to_unit_ticks() {
        let unit = SignedDuration::from_minutes(5.0);
        let axis = Axis::time_span(unit, |span| span.to_string());
        assert_eq!(axis.unit_width, unit.ticks() as f64);
        assert_eq!(axis.min_step, unit.ticks() as f64);
    }

    #[test]
    fn time_span_axis_labeler_formats_ticks_as_time_span() {
        let axis = Axis::time_span(SignedDuration::from_seconds(1.0), |span| {
            format!("{}m", span.total_minutes())
        });
        let one_minute = SignedDuration::from_minutes(1.0).ticks() as f64;
        assert_eq!(axis.label(one_minute), "1m");
    }

    #[test]
    fn time_span_axis_labeler_clamps_negative_ticks_to_zero() {
        let axis = Axis::time_span(SignedDuration::from_milliseconds(1.0), |span| {
            span.ticks().to_string()
        });
        assert_eq!(axis.label(-1000.0), "0");
    }

    #[test]
    fn date_time_axis_labels_the_dates_of_its_ticks() {
        let axis = Axis::date_time(SignedDuration::from_days(1.0), |date| {
            date.try_to_string_format("MM dd", &CultureInfo::invariant_culture())
                .unwrap()
        });
        let day = SignedDuration::from_days(1.0).ticks() as f64;
        assert_eq!((axis.unit_width, axis.min_step), (day, day));
        let date = DateTime::new(2021, 1, 6);
        assert_eq!(axis.label(date.ticks() as f64), "01 06");
        assert_eq!(as_date(-5.0), DateTime::MIN_VALUE);
        assert_eq!(as_date(f64::MAX), DateTime::MAX_VALUE);
        // The labels of an axis win over its labeler, as in the source's GetActualLabeler.
        let named = Axis {
            labels: vec!["a".into()],
            ..axis
        };
        assert_eq!(named.label(0.0), "a");
    }
}
