//! The time type of the animation system.
//!
//! Animation timing is expressed in signed 100-nanosecond ticks: clocks hand
//! out the time elapsed since they started, tests step clocks backwards, and
//! a negative `Delay` or `Duration` has to be representable so that it can be
//! rejected when an animation runs rather than when it is declared. A
//! [`std::time::Duration`] cannot hold any of that, so the animation API uses
//! [`TimeSpan`] and converts from and to `Duration` at its edges.

use crate::utilities::FormatError;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};
use std::str::FromStr;
use std::time::Duration;

/// A signed time interval with a resolution of 100 nanoseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeSpan {
    ticks: i64,
}

impl TimeSpan {
    /// The number of ticks in one millisecond.
    pub const TICKS_PER_MILLISECOND: i64 = 10_000;
    /// The number of ticks in one second.
    pub const TICKS_PER_SECOND: i64 = 10_000_000;
    /// The number of ticks in one minute.
    pub const TICKS_PER_MINUTE: i64 = 600_000_000;
    /// The number of ticks in one hour.
    pub const TICKS_PER_HOUR: i64 = 36_000_000_000;
    /// The number of ticks in one day.
    pub const TICKS_PER_DAY: i64 = 864_000_000_000;

    /// The zero interval.
    pub const ZERO: TimeSpan = TimeSpan { ticks: 0 };
    /// The largest interval.
    pub const MAX_VALUE: TimeSpan = TimeSpan { ticks: i64::MAX };
    /// The smallest (most negative) interval.
    pub const MIN_VALUE: TimeSpan = TimeSpan { ticks: i64::MIN };

    /// Creates an interval from a number of 100-nanosecond ticks.
    #[inline]
    pub const fn from_ticks(ticks: i64) -> Self {
        Self { ticks }
    }

    /// Creates an interval from a number of seconds, rounded to the nearest
    /// tick. Panics if the value is not a number or out of range.
    pub fn from_seconds(value: f64) -> Self {
        Self::interval(value, Self::TICKS_PER_SECOND as f64)
    }

    /// Creates an interval from a number of milliseconds, rounded to the
    /// nearest tick. Panics if the value is not a number or out of range.
    pub fn from_milliseconds(value: f64) -> Self {
        Self::interval(value, Self::TICKS_PER_MILLISECOND as f64)
    }

    /// Creates an interval from a number of minutes.
    pub fn from_minutes(value: f64) -> Self {
        Self::interval(value, Self::TICKS_PER_MINUTE as f64)
    }

    fn interval(value: f64, scale: f64) -> Self {
        assert!(!value.is_nan(), "TimeSpan does not accept floating point Not-a-Number values.");
        let ticks = value * scale;
        assert!(
            ticks <= i64::MAX as f64 && ticks >= i64::MIN as f64,
            "TimeSpan overflowed because the duration is too long."
        );
        Self { ticks: ticks.round() as i64 }
    }

    /// The number of ticks.
    #[inline]
    pub const fn ticks(self) -> i64 {
        self.ticks
    }

    /// The interval in whole and fractional seconds.
    #[inline]
    pub fn total_seconds(self) -> f64 {
        self.ticks as f64 / Self::TICKS_PER_SECOND as f64
    }

    /// The interval in whole and fractional milliseconds.
    #[inline]
    pub fn total_milliseconds(self) -> f64 {
        self.ticks as f64 / Self::TICKS_PER_MILLISECOND as f64
    }

    /// The interval as a [`Duration`]; `None` when it is negative.
    pub fn to_duration(self) -> Option<Duration> {
        if self.ticks < 0 {
            return None;
        }
        let ticks = self.ticks as u64;
        Some(Duration::new(ticks / Self::TICKS_PER_SECOND as u64, ((ticks % Self::TICKS_PER_SECOND as u64) * 100) as u32))
    }

    /// Parses an interval in the invariant format
    /// `[-]{ d | [d.]hh:mm[:ss[.fffffff]] | d:hh:mm:ss[.fffffff] }`.
    pub fn parse(s: &str) -> Result<TimeSpan, FormatError> {
        let invalid = || FormatError::from_string(format!("Invalid TimeSpan string: \"{s}\"."));
        let text = s.trim();
        let (negative, body) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        if body.is_empty() {
            return Err(invalid());
        }
        let number = |part: &str| -> Result<i64, FormatError> {
            if part.is_empty() || part.len() > 9 || !part.bytes().all(|b| b.is_ascii_digit()) {
                return Err(invalid());
            }
            part.parse::<i64>().map_err(|_| invalid())
        };

        let parts: Vec<&str> = body.split(':').collect();
        let (mut days, hours, minutes, seconds_part): (i64, i64, i64, Option<&str>) = match parts.as_slice() {
            [days] => (number(days)?, 0, 0, None),
            [hours, minutes] => {
                let (days, hours) = Self::split_days(hours, &number)?;
                (days, hours, number(minutes)?, None)
            }
            [hours, minutes, seconds] => {
                let (days, hours) = Self::split_days(hours, &number)?;
                (days, hours, number(minutes)?, Some(*seconds))
            }
            [days, hours, minutes, seconds] => (number(days)?, number(hours)?, number(minutes)?, Some(*seconds)),
            _ => return Err(invalid()),
        };
        let (seconds, fraction_ticks) = match seconds_part {
            None => (0, 0),
            Some(part) => match part.split_once('.') {
                None => (number(part)?, 0),
                Some((whole, fraction)) => {
                    if fraction.is_empty() || fraction.len() > 7 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(invalid());
                    }
                    let mut ticks = fraction.parse::<i64>().map_err(|_| invalid())?;
                    for _ in fraction.len()..7 {
                        ticks *= 10;
                    }
                    // The seconds may be left out before the fraction ("0:0:.075").
                    (if whole.is_empty() { 0 } else { number(whole)? }, ticks)
                }
            },
        };
        if hours > 23 || minutes > 59 || seconds > 59 {
            return Err(FormatError::from_string(format!(
                "The TimeSpan string \"{s}\" could not be parsed because at least one of the numeric components is out of range."
            )));
        }
        if days > 10_675_199 {
            days = i64::MAX / Self::TICKS_PER_DAY + 1;
        }
        let ticks = days
            .checked_mul(Self::TICKS_PER_DAY)
            .and_then(|t| t.checked_add(hours * Self::TICKS_PER_HOUR))
            .and_then(|t| t.checked_add(minutes * Self::TICKS_PER_MINUTE))
            .and_then(|t| t.checked_add(seconds * Self::TICKS_PER_SECOND))
            .and_then(|t| t.checked_add(fraction_ticks))
            .ok_or_else(|| {
                FormatError::from_string(format!(
                    "The TimeSpan string \"{s}\" could not be parsed because at least one of the numeric components is out of range."
                ))
            })?;
        Ok(TimeSpan { ticks: if negative { -ticks } else { ticks } })
    }

    fn split_days(part: &str, number: &dyn Fn(&str) -> Result<i64, FormatError>) -> Result<(i64, i64), FormatError> {
        match part.split_once('.') {
            Some((days, hours)) => Ok((number(days)?, number(hours)?)),
            None => Ok((0, number(part)?)),
        }
    }
}

/// Members used by the date and time types and the pickers.
impl TimeSpan {
    /// The interval of the given hours, minutes and seconds (the C#
    /// constructor `TimeSpan(int, int, int)`). Panics on overflow.
    pub fn from_hms(hours: i32, minutes: i32, seconds: i32) -> Self {
        Self::from_dhms_milliseconds(0, hours, minutes, seconds, 0)
    }

    /// The interval of the given days, hours, minutes and seconds (the C#
    /// constructor `TimeSpan(int, int, int, int)`). Panics on overflow.
    pub fn from_dhms(days: i32, hours: i32, minutes: i32, seconds: i32) -> Self {
        Self::from_dhms_milliseconds(days, hours, minutes, seconds, 0)
    }

    /// The interval of the given days, hours, minutes, seconds and
    /// milliseconds (the C# constructor `TimeSpan(int, int, int, int, int)`).
    /// Panics on overflow.
    pub fn from_dhms_milliseconds(days: i32, hours: i32, minutes: i32, seconds: i32, milliseconds: i32) -> Self {
        let total_milliseconds = (days as i64 * 86_400 + hours as i64 * 3_600 + minutes as i64 * 60 + seconds as i64) * 1_000
            + milliseconds as i64;
        let ticks = total_milliseconds
            .checked_mul(Self::TICKS_PER_MILLISECOND)
            .expect("TimeSpan overflowed because the duration is too long.");
        Self { ticks }
    }

    /// Creates an interval from a number of hours.
    pub fn from_hours(value: f64) -> Self {
        Self::interval(value, Self::TICKS_PER_HOUR as f64)
    }

    /// Creates an interval from a number of days.
    pub fn from_days(value: f64) -> Self {
        Self::interval(value, Self::TICKS_PER_DAY as f64)
    }

    /// The whole days of the interval (negative for a negative interval).
    #[inline]
    pub const fn days(self) -> i32 {
        (self.ticks / Self::TICKS_PER_DAY) as i32
    }

    /// The hours component, -23..=23.
    #[inline]
    pub const fn hours(self) -> i32 {
        (self.ticks / Self::TICKS_PER_HOUR % 24) as i32
    }

    /// The minutes component, -59..=59.
    #[inline]
    pub const fn minutes(self) -> i32 {
        (self.ticks / Self::TICKS_PER_MINUTE % 60) as i32
    }

    /// The seconds component, -59..=59.
    #[inline]
    pub const fn seconds(self) -> i32 {
        (self.ticks / Self::TICKS_PER_SECOND % 60) as i32
    }

    /// The milliseconds component, -999..=999.
    #[inline]
    pub const fn milliseconds(self) -> i32 {
        (self.ticks / Self::TICKS_PER_MILLISECOND % 1000) as i32
    }

    /// The interval in whole and fractional days.
    #[inline]
    pub fn total_days(self) -> f64 {
        self.ticks as f64 / Self::TICKS_PER_DAY as f64
    }

    /// The interval in whole and fractional hours.
    #[inline]
    pub fn total_hours(self) -> f64 {
        self.ticks as f64 / Self::TICKS_PER_HOUR as f64
    }

    /// The interval in whole and fractional minutes.
    #[inline]
    pub fn total_minutes(self) -> f64 {
        self.ticks as f64 / Self::TICKS_PER_MINUTE as f64
    }

    /// The absolute value of the interval. Panics for [`TimeSpan::MIN_VALUE`].
    pub fn duration(self) -> TimeSpan {
        TimeSpan { ticks: self.ticks.checked_abs().expect("TimeSpan overflowed because the duration is too long.") }
    }

    /// Formats with a standard (`c`, `t`, `T`, `g`, `G`) or custom .NET
    /// `TimeSpan` format string; an empty format is `c`. Panics when the
    /// format string is invalid (.NET throws `FormatException`).
    ///
    /// The `g` and `G` formats always use `.` as the decimal separator; .NET
    /// uses the one of the culture.
    #[track_caller]
    pub fn to_string_format(self, format: &str) -> String {
        match self.try_to_string_format(format) {
            Ok(text) => text,
            Err(error) => panic!("{error}"),
        }
    }

    /// Like [`to_string_format`](Self::to_string_format), but gives the error
    /// of an invalid format string.
    pub fn try_to_string_format(self, format: &str) -> Result<String, FormatError> {
        use std::fmt::Write;

        let invalid = || FormatError::new("Input string was not in a correct format.");
        let negative = self.ticks < 0;
        let ticks = self.ticks.unsigned_abs();
        let days = ticks / Self::TICKS_PER_DAY as u64;
        let hours = ticks / Self::TICKS_PER_HOUR as u64 % 24;
        let minutes = ticks / Self::TICKS_PER_MINUTE as u64 % 60;
        let seconds = ticks / Self::TICKS_PER_SECOND as u64 % 60;
        let fraction = ticks % Self::TICKS_PER_SECOND as u64;

        let mut out = String::new();
        let mut chars = format.chars();
        match (chars.next(), chars.next()) {
            (None, _) | (Some('c' | 't' | 'T'), None) => return Ok(self.to_string()),
            (Some('g'), None) => {
                if negative {
                    out.push('-');
                }
                if days != 0 {
                    let _ = write!(out, "{days}:");
                }
                let _ = write!(out, "{hours}:{minutes:02}:{seconds:02}");
                if fraction != 0 {
                    let digits = format!("{fraction:07}");
                    let _ = write!(out, ".{}", digits.trim_end_matches('0'));
                }
                return Ok(out);
            }
            (Some('G'), None) => {
                if negative {
                    out.push('-');
                }
                let _ = write!(out, "{days}:{hours:02}:{minutes:02}:{seconds:02}.{fraction:07}");
                return Ok(out);
            }
            (Some(_), None) => return Err(invalid()),
            _ => {}
        }

        // A custom format: every character other than a specifier has to be
        // quoted or escaped; the sign is never written.
        let bytes = format.as_bytes();
        let mut index = 0;
        let mut single = false;
        while index < bytes.len() {
            let byte = bytes[index];
            let count = if single { 1 } else { bytes[index..].iter().take_while(|other| **other == byte).count() };
            single = false;
            match byte {
                b'd' if count <= 8 => {
                    let _ = write!(out, "{days:0count$}");
                }
                b'h' if count <= 2 => {
                    let _ = write!(out, "{hours:0count$}");
                }
                b'm' if count <= 2 => {
                    let _ = write!(out, "{minutes:0count$}");
                }
                b's' if count <= 2 => {
                    let _ = write!(out, "{seconds:0count$}");
                }
                b'f' if count <= 7 => {
                    let value = fraction / 10_u64.pow((7 - count) as u32);
                    let _ = write!(out, "{value:0count$}");
                }
                b'F' if count <= 7 => {
                    let mut value = fraction / 10_u64.pow((7 - count) as u32);
                    let mut digits = count;
                    while digits > 0 && value % 10 == 0 {
                        value /= 10;
                        digits -= 1;
                    }
                    if digits > 0 {
                        let _ = write!(out, "{value:0digits$}");
                    }
                }
                b'\'' | b'"' => {
                    // A quoted literal; a backslash in it escapes the next
                    // character.
                    let mut rest = format[index + 1..].char_indices();
                    let end = loop {
                        match rest.next() {
                            Some((offset, c)) if c == byte as char => break index + 1 + offset,
                            Some((_, '\\')) => out.push(rest.next().ok_or_else(invalid)?.1),
                            Some((_, c)) => out.push(c),
                            None => return Err(invalid()),
                        }
                    };
                    index = end + 1;
                    continue;
                }
                b'%' => {
                    // The next character is a custom specifier on its own: a
                    // format of that one character, which a quote or a
                    // backslash is not.
                    match bytes.get(index + 1) {
                        Some(next) if !matches!(*next, b'%' | b'\'' | b'"' | b'\\') => single = true,
                        _ => return Err(invalid()),
                    }
                    index += 1;
                    continue;
                }
                b'\\' => {
                    let escaped = format[index + 1..].chars().next().ok_or_else(invalid)?;
                    out.push(escaped);
                    index += 1 + escaped.len_utf8();
                    continue;
                }
                _ => return Err(invalid()),
            }
            index += count;
        }
        Ok(out)
    }
}

impl From<Duration> for TimeSpan {
    /// Converts a duration, truncating to whole ticks and saturating at
    /// [`TimeSpan::MAX_VALUE`].
    fn from(value: Duration) -> Self {
        let ticks = value.as_nanos() / 100;
        TimeSpan { ticks: i64::try_from(ticks).unwrap_or(i64::MAX) }
    }
}

impl FromStr for TimeSpan {
    type Err = FormatError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TimeSpan::parse(s)
    }
}

impl fmt::Display for TimeSpan {
    /// Formats as `[-][d.]hh:mm:ss[.fffffff]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let negative = self.ticks < 0;
        let ticks = self.ticks.unsigned_abs();
        let days = ticks / Self::TICKS_PER_DAY as u64;
        let hours = ticks / Self::TICKS_PER_HOUR as u64 % 24;
        let minutes = ticks / Self::TICKS_PER_MINUTE as u64 % 60;
        let seconds = ticks / Self::TICKS_PER_SECOND as u64 % 60;
        let fraction = ticks % Self::TICKS_PER_SECOND as u64;
        if negative {
            f.write_str("-")?;
        }
        if days != 0 {
            write!(f, "{days}.")?;
        }
        write!(f, "{hours:02}:{minutes:02}:{seconds:02}")?;
        if fraction != 0 {
            write!(f, ".{fraction:07}")?;
        }
        Ok(())
    }
}

impl Add for TimeSpan {
    type Output = TimeSpan;
    #[inline]
    fn add(self, rhs: TimeSpan) -> TimeSpan {
        TimeSpan { ticks: self.ticks.checked_add(rhs.ticks).expect("TimeSpan overflowed because the duration is too long.") }
    }
}

impl Sub for TimeSpan {
    type Output = TimeSpan;
    #[inline]
    fn sub(self, rhs: TimeSpan) -> TimeSpan {
        TimeSpan { ticks: self.ticks.checked_sub(rhs.ticks).expect("TimeSpan overflowed because the duration is too long.") }
    }
}

impl AddAssign for TimeSpan {
    #[inline]
    fn add_assign(&mut self, rhs: TimeSpan) {
        *self = *self + rhs;
    }
}

impl SubAssign for TimeSpan {
    #[inline]
    fn sub_assign(&mut self, rhs: TimeSpan) {
        *self = *self - rhs;
    }
}

impl Neg for TimeSpan {
    type Output = TimeSpan;
    #[inline]
    fn neg(self) -> TimeSpan {
        TimeSpan { ticks: self.ticks.checked_neg().expect("TimeSpan overflowed because the duration is too long.") }
    }
}

impl Mul<f64> for TimeSpan {
    type Output = TimeSpan;
    /// Scales the interval, rounding to the nearest tick.
    fn mul(self, rhs: f64) -> TimeSpan {
        TimeSpan::interval(self.ticks as f64, rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_seconds_rounds_to_ticks() {
        assert_eq!(TimeSpan::from_seconds(1.0).ticks(), 10_000_000);
        assert_eq!(TimeSpan::from_seconds(-0.5).ticks(), -5_000_000);
        assert_eq!(TimeSpan::from_milliseconds(10.0).ticks(), 100_000);
        assert_eq!(TimeSpan::from_seconds(0.5).total_seconds(), 0.5);
    }

    #[test]
    fn parse_accepts_the_invariant_formats() {
        assert_eq!(TimeSpan::parse("0:0:1").unwrap(), TimeSpan::from_seconds(1.0));
        assert_eq!(TimeSpan::parse("00:00:00.3").unwrap(), TimeSpan::from_milliseconds(300.0));
        assert_eq!(TimeSpan::parse("00:00:10.0153932").unwrap().ticks(), 100_153_932);
        assert_eq!(TimeSpan::parse("1").unwrap().ticks(), TimeSpan::TICKS_PER_DAY);
        assert_eq!(TimeSpan::parse("1:30").unwrap().ticks(), TimeSpan::TICKS_PER_HOUR + 30 * TimeSpan::TICKS_PER_MINUTE);
        assert_eq!(TimeSpan::parse("1.02:03:04").unwrap().ticks(), TimeSpan::parse("1:2:3:4").unwrap().ticks());
        assert_eq!(TimeSpan::parse(" -0:0:2 ").unwrap(), TimeSpan::from_seconds(-2.0));
        // The seconds may be left out before their fraction.
        assert_eq!(TimeSpan::parse("0:0:.075").unwrap().ticks(), 750_000);
    }

    #[test]
    fn parse_rejects_malformed_input() {
        for input in ["", "abc", "1:2:3:4:5", "0:60:0", "24:00", "0:0:1.", "0:0:1.12345678", "1,5"] {
            assert!(TimeSpan::parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn display_round_trips() {
        for input in ["00:00:01", "1.02:03:04", "-00:00:02.5000000", "00:00:10.0153932"] {
            assert_eq!(TimeSpan::parse(input).unwrap().to_string(), input);
        }
    }

    /// The expected texts are the ones .NET gives (`TimeSpan.ToString(format,
    /// CultureInfo.InvariantCulture)`); `None` is a `FormatException`.
    #[test]
    fn custom_formats_match_net() {
        let spans = [
            TimeSpan::from_hms(0, 0, 0),
            TimeSpan::from_hms(9, 4, 5),
            TimeSpan::from_hms(15, 4, 5),
            TimeSpan::from_dhms(1, 2, 3, 4),
            -TimeSpan::from_hms(3, 4, 5),
            TimeSpan::from_dhms_milliseconds(0, 23, 59, 59, 999),
        ];
        let rows: [(&str, Option<[&str; 6]>); 20] = [
            ("%h", Some(["0", "9", "15", "2", "3", "23"])),
            ("hh", Some(["00", "09", "15", "02", "03", "23"])),
            ("h", None),
            ("%m", Some(["0", "4", "4", "3", "4", "59"])),
            ("mm", Some(["00", "04", "04", "03", "04", "59"])),
            ("ss", Some(["00", "05", "05", "04", "05", "59"])),
            ("%s", Some(["0", "5", "5", "4", "5", "59"])),
            ("%'a'", None),
            ("%%", None),
            ("h\\:mm", Some(["0:00", "9:04", "15:04", "2:03", "3:04", "23:59"])),
            ("hh':'mm", Some(["00:00", "09:04", "15:04", "02:03", "03:04", "23:59"])),
            ("%d", Some(["0", "0", "0", "1", "0", "0"])),
            ("hhh", None),
            ("%hh", Some(["00", "99", "1515", "22", "33", "2323"])),
            ("h mm", None),
            ("\\h", Some(["h"; 6])),
            ("'ab", None),
            ("%\\h", None),
            ("ff", Some(["00", "00", "00", "00", "00", "99"])),
            ("%F", Some(["", "", "", "", "", "9"])),
        ];
        for (format, expected) in rows {
            for (index, span) in spans.iter().enumerate() {
                let actual = span.try_to_string_format(format).ok();
                assert_eq!(expected.map(|texts| texts[index].to_string()), actual, "{format:?} of {span}");
            }
        }

        // A backslash escapes inside a quoted literal too.
        assert_eq!("h'm 09", TimeSpan::from_hms(9, 4, 5).to_string_format("'h\\'m 'hh"));
        assert!(TimeSpan::from_hms(9, 4, 5).try_to_string_format("'h\\'").is_err());
    }

    #[test]
    fn converts_from_and_to_duration() {
        assert_eq!(TimeSpan::from(Duration::from_millis(1500)), TimeSpan::from_seconds(1.5));
        assert_eq!(TimeSpan::from_seconds(1.5).to_duration(), Some(Duration::from_millis(1500)));
        assert_eq!(TimeSpan::from_seconds(-1.0).to_duration(), None);
    }
}
