//! A point in time as a date and time of day together with its offset from
//! UTC (.NET `System.DateTimeOffset`).

use super::date_time_format;
use super::{CultureInfo, DateTime, DateTimeFormatProvider, DateTimeKind, DayOfWeek, FormatError, TimeZoneInfo};
use crate::animation::TimeSpan;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Sub, SubAssign};

const OUT_OF_RANGE: &str = "The UTC time represented when the offset is applied must be between year 0 and 10,000.";

/// A date and time of day relative to UTC.
///
/// Equality, ordering and hashing consider the UTC instant only: two values
/// with different offsets that name the same instant are equal, as in .NET
/// (use [`equals_exact`](Self::equals_exact) to compare the offsets too).
#[derive(Clone, Copy)]
pub struct DateTimeOffset {
    /// The UTC instant; its kind is always unspecified.
    utc: DateTime,
    offset_minutes: i16,
}

impl DateTimeOffset {
    /// The smallest value, 0001-01-01T00:00:00 +00:00.
    pub const MIN_VALUE: DateTimeOffset = DateTimeOffset { utc: DateTime::MIN_VALUE, offset_minutes: 0 };
    /// The largest value, 9999-12-31T23:59:59.9999999 +00:00.
    pub const MAX_VALUE: DateTimeOffset = DateTimeOffset { utc: DateTime::MAX_VALUE, offset_minutes: 0 };
    /// 1970-01-01T00:00:00 +00:00.
    pub const UNIX_EPOCH: DateTimeOffset =
        DateTimeOffset { utc: DateTime::UNIX_EPOCH.specify_kind(DateTimeKind::Unspecified), offset_minutes: 0 };

    fn validate_offset(offset: TimeSpan) -> Result<i16, &'static str> {
        let ticks = offset.ticks();
        if ticks % TimeSpan::TICKS_PER_MINUTE != 0 {
            return Err("Offset must be specified in whole minutes. (Parameter 'offset')");
        }
        if ticks.abs() > TimeZoneInfo::MAX_OFFSET.ticks() {
            return Err("Offset must be within plus or minus 14 hours. (Parameter 'offset')");
        }
        Ok((ticks / TimeSpan::TICKS_PER_MINUTE) as i16)
    }

    fn from_clock(clock: DateTime, offset: TimeSpan) -> Result<DateTimeOffset, &'static str> {
        let offset_minutes = Self::validate_offset(offset)?;
        let utc = DateTime::try_from_ticks(clock.ticks() - offset.ticks(), DateTimeKind::Unspecified).ok_or(OUT_OF_RANGE)?;
        Ok(DateTimeOffset { utc, offset_minutes })
    }

    /// The given clock date and time at `offset` from UTC. Panics when the
    /// date and time are invalid, the offset is not a whole number of
    /// minutes within 14 hours of UTC, or the UTC instant is out of range.
    #[track_caller]
    pub fn new(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32, offset: TimeSpan) -> DateTimeOffset {
        let clock = DateTime::new_with_time(year, month, day, hour, minute, second);
        Self::from_clock(clock, offset).unwrap_or_else(|message| panic!("{message}"))
    }

    /// Like [`new`](Self::new); `None` instead of a panic.
    pub fn try_new(
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: i32,
        offset: TimeSpan,
    ) -> Option<DateTimeOffset> {
        let clock = DateTime::try_new_with_time(year, month, day, hour, minute, second)?;
        Self::from_clock(clock, offset).ok()
    }

    /// The clock time with `ticks` ticks at `offset` from UTC. Panics when
    /// the ticks, the offset or the UTC instant are out of range.
    #[track_caller]
    pub fn from_ticks(ticks: i64, offset: TimeSpan) -> DateTimeOffset {
        Self::from_clock(DateTime::from_ticks(ticks), offset).unwrap_or_else(|message| panic!("{message}"))
    }

    /// The instant of a [`DateTime`]: a UTC value gets the offset zero, a
    /// local or unspecified value the local offset at that time
    /// (.NET `new DateTimeOffset(DateTime)`). Panics when the UTC instant is
    /// out of range.
    #[track_caller]
    pub fn from_date_time(date_time: DateTime) -> DateTimeOffset {
        let offset = if date_time.kind() == DateTimeKind::Utc {
            TimeSpan::ZERO
        } else {
            TimeZoneInfo::get_utc_offset_of_local_time(date_time)
        };
        Self::from_clock(date_time, offset).unwrap_or_else(|message| panic!("{message}"))
    }

    /// The clock time `date_time` at `offset` from UTC
    /// (.NET `new DateTimeOffset(DateTime, TimeSpan)`). Panics when the offset
    /// is invalid, when a UTC value is given a non-zero offset, when a local
    /// value is given an offset other than the local one, or when the UTC
    /// instant is out of range.
    #[track_caller]
    pub fn from_date_time_offset(date_time: DateTime, offset: TimeSpan) -> DateTimeOffset {
        match date_time.kind() {
            DateTimeKind::Utc => assert!(offset == TimeSpan::ZERO, "The UTC Offset for Utc DateTime instances must be 0. (Parameter 'offset')"),
            DateTimeKind::Local => assert!(
                offset == TimeZoneInfo::get_utc_offset_of_local_time(date_time),
                "The UTC Offset of the local dateTime parameter does not match the offset argument. (Parameter 'offset')"
            ),
            DateTimeKind::Unspecified => {}
        }
        Self::from_clock(date_time, offset).unwrap_or_else(|message| panic!("{message}"))
    }

    /// The current date and time with the local offset.
    pub fn now() -> DateTimeOffset {
        Self::utc_now().to_local_time()
    }

    /// The current date and time with the offset zero.
    pub fn utc_now() -> DateTimeOffset {
        DateTimeOffset { utc: DateTime::utc_now().specify_kind(DateTimeKind::Unspecified), offset_minutes: 0 }
    }

    // --- components ---

    /// The offset from UTC.
    #[inline]
    pub const fn offset(self) -> TimeSpan {
        TimeSpan::from_ticks(self.offset_minutes as i64 * TimeSpan::TICKS_PER_MINUTE)
    }

    /// The clock date and time, of unspecified kind (.NET `DateTime`).
    #[inline]
    pub const fn date_time(self) -> DateTime {
        // The constructors keep both the UTC instant and the clock time in range.
        DateTime::from_ticks(self.utc.ticks() + self.offset().ticks())
    }

    /// The UTC date and time, of kind UTC (.NET `UtcDateTime`).
    #[inline]
    pub const fn utc_date_time(self) -> DateTime {
        self.utc.specify_kind(DateTimeKind::Utc)
    }

    /// The date and time in local time, of kind local (.NET `LocalDateTime`).
    pub fn local_date_time(self) -> DateTime {
        self.utc_date_time().to_local_time()
    }

    /// The clock date at midnight, of unspecified kind.
    #[inline]
    pub const fn date(self) -> DateTime {
        self.date_time().date()
    }

    /// The clock time elapsed since midnight.
    #[inline]
    pub const fn time_of_day(self) -> TimeSpan {
        self.date_time().time_of_day()
    }

    /// The ticks of the clock date and time.
    #[inline]
    pub const fn ticks(self) -> i64 {
        self.date_time().ticks()
    }

    /// The ticks of the UTC date and time.
    #[inline]
    pub const fn utc_ticks(self) -> i64 {
        self.utc.ticks()
    }

    /// The year of the clock date.
    pub fn year(self) -> i32 {
        self.date_time().year()
    }

    /// The month of the clock date, 1..=12.
    pub fn month(self) -> i32 {
        self.date_time().month()
    }

    /// The day of the month of the clock date, 1..=31.
    pub fn day(self) -> i32 {
        self.date_time().day()
    }

    /// The day of the year of the clock date, 1..=366.
    pub fn day_of_year(self) -> i32 {
        self.date_time().day_of_year()
    }

    /// The day of the week of the clock date.
    pub fn day_of_week(self) -> DayOfWeek {
        self.date_time().day_of_week()
    }

    /// The hour of the clock time, 0..=23.
    pub const fn hour(self) -> i32 {
        self.date_time().hour()
    }

    /// The minute of the clock time, 0..=59.
    pub const fn minute(self) -> i32 {
        self.date_time().minute()
    }

    /// The second of the clock time, 0..=59.
    pub const fn second(self) -> i32 {
        self.date_time().second()
    }

    /// The millisecond of the clock time, 0..=999.
    pub const fn millisecond(self) -> i32 {
        self.date_time().millisecond()
    }

    // --- arithmetic ---

    #[track_caller]
    fn with_clock(self, clock: DateTime) -> DateTimeOffset {
        Self::from_clock(clock, self.offset()).unwrap_or_else(|message| panic!("{message}"))
    }

    /// The value `value` later, with the same offset. Panics when out of range.
    #[track_caller]
    pub fn add(self, value: TimeSpan) -> DateTimeOffset {
        self.with_clock(self.date_time().add(value))
    }

    /// The value `value` earlier, with the same offset. Panics when out of range.
    #[track_caller]
    pub fn subtract(self, value: TimeSpan) -> DateTimeOffset {
        self.with_clock(self.date_time().subtract(value))
    }

    /// The interval from `other` to this value (by UTC instants).
    #[inline]
    pub fn subtract_date_time_offset(self, other: DateTimeOffset) -> TimeSpan {
        self.utc - other.utc
    }

    /// The value `value` days later. Panics when out of range.
    #[track_caller]
    pub fn add_days(self, value: f64) -> DateTimeOffset {
        self.with_clock(self.date_time().add_days(value))
    }

    /// The value `months` months later (see [`DateTime::add_months`]).
    #[track_caller]
    pub fn add_months(self, months: i32) -> DateTimeOffset {
        self.with_clock(self.date_time().add_months(months))
    }

    /// The value `years` years later (see [`DateTime::add_years`]).
    #[track_caller]
    pub fn add_years(self, years: i32) -> DateTimeOffset {
        self.with_clock(self.date_time().add_years(years))
    }

    // --- conversion ---

    /// The same instant at another offset. Panics when the offset is invalid
    /// or the clock time at that offset is out of range.
    #[track_caller]
    pub fn to_offset(self, offset: TimeSpan) -> DateTimeOffset {
        let offset_minutes = Self::validate_offset(offset).unwrap_or_else(|message| panic!("{message}"));
        assert!(DateTime::try_from_ticks(self.utc.ticks() + offset.ticks(), DateTimeKind::Unspecified).is_some(), "{OUT_OF_RANGE}");
        DateTimeOffset { utc: self.utc, offset_minutes }
    }

    /// The same instant at the local offset. A clock time that would leave
    /// the range keeps the instant and gets the offset zero.
    pub fn to_local_time(self) -> DateTimeOffset {
        let offset = TimeZoneInfo::get_local_utc_offset(self.utc_date_time());
        match DateTime::try_from_ticks(self.utc.ticks() + offset.ticks(), DateTimeKind::Unspecified) {
            Some(_) => DateTimeOffset { utc: self.utc, offset_minutes: (offset.ticks() / TimeSpan::TICKS_PER_MINUTE) as i16 },
            None => DateTimeOffset { utc: self.utc, offset_minutes: 0 },
        }
    }

    /// The same instant at the offset zero.
    #[inline]
    pub const fn to_universal_time(self) -> DateTimeOffset {
        DateTimeOffset { utc: self.utc, offset_minutes: 0 }
    }

    // --- comparison ---

    /// Compares two values by their UTC instants: -1, 0 or 1.
    pub fn compare(first: DateTimeOffset, second: DateTimeOffset) -> i32 {
        DateTime::compare(first.utc, second.utc)
    }

    /// Compares this value with `other` by their UTC instants: -1, 0 or 1.
    pub fn compare_to(self, other: DateTimeOffset) -> i32 {
        Self::compare(self, other)
    }

    /// Whether both the UTC instant and the offset are the same.
    pub fn equals_exact(self, other: DateTimeOffset) -> bool {
        self.utc == other.utc && self.offset_minutes == other.offset_minutes
    }

    // --- formatting ---

    /// Formats with a standard or custom .NET date and time format string
    /// and the conventions of `provider`. An empty format gives the short
    /// date, the long time and the offset (`"G"` followed by `zzz`). Panics
    /// when the format string is invalid (.NET throws `FormatException`).
    #[track_caller]
    pub fn to_string_format(self, format: &str, provider: &(impl DateTimeFormatProvider + ?Sized)) -> String {
        match self.try_to_string_format(format, provider) {
            Ok(text) => text,
            Err(error) => panic!("{error}"),
        }
    }

    /// Like [`to_string_format`](Self::to_string_format), but gives the error
    /// of an invalid format string.
    pub fn try_to_string_format(
        self,
        format: &str,
        provider: &(impl DateTimeFormatProvider + ?Sized),
    ) -> Result<String, FormatError> {
        date_time_format::format(self.date_time(), Some(self.offset()), format, &provider.get_date_time_format())
    }

    /// Formats with a format string and the conventions of the current
    /// culture (.NET `ToString(string)`). Panics when the format is invalid.
    #[track_caller]
    pub fn to_string_with(self, format: &str) -> String {
        self.to_string_format(format, &CultureInfo::current_culture())
    }

    /// Formats with the default format and the conventions of `provider`
    /// (.NET `ToString(IFormatProvider)`).
    pub fn to_string_provider(self, provider: &(impl DateTimeFormatProvider + ?Sized)) -> String {
        self.to_string_format("", provider)
    }
}

impl std::str::FromStr for DateTimeOffset {
    type Err = FormatError;

    /// [`DateTimeOffset::parse`]: the invariant culture.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        DateTimeOffset::parse(text, &CultureInfo::invariant_culture())
    }
}

impl DateTimeOffset {
    // --- parsing ---

    /// Parses a date and time with an optional offset written in the
    /// conventions of `provider` (.NET
    /// `DateTimeOffset.Parse(string, IFormatProvider)`): the forms
    /// [`DateTime::parse`] accepts. The value has the offset the text
    /// states; a text without an offset is a local time and gets the local
    /// offset at that time, and a text without a date the current local
    /// date, as in .NET. The offset is `Z`, `GMT`, or a signed `hh:mm`,
    /// `hhmm` or `h[h]` after the time or the date. An offset of more than
    /// 14 hours and an instant outside of the years 1 to 9999 are errors.
    ///
    /// Checked against results recorded from .NET for the invariant culture
    /// (`date_time_tests.rs`, `date_time_offset_parse_matches_net`). Not
    /// supported, as for [`DateTime::parse`]: what the heuristic parser of
    /// .NET accepts beyond those forms (other time zone names, era names,
    /// genitive month names), the `DateTimeStyles` overloads and
    /// `ParseExact`.
    pub fn parse(text: &str, provider: &(impl DateTimeFormatProvider + ?Sized)) -> Result<DateTimeOffset, FormatError> {
        let parsed =
            date_time_format::parse(text, &provider.get_date_time_format(), super::DateTimeStyles::NONE)?;
        let clock = parsed.date_time.specify_kind(DateTimeKind::Unspecified);
        let offset = match parsed.offset {
            Some(offset) => offset,
            None => TimeZoneInfo::get_utc_offset_of_local_time(clock),
        };
        if Self::validate_offset(offset).is_err() {
            return Err(FormatError::from_string(format!(
                "The time zone offset of string '{text}' must be within plus or minus 14 hours."
            )));
        }
        Self::from_clock(clock, offset).map_err(|_| {
            FormatError::from_string(format!(
                "The UTC representation of the date '{text}' falls outside the year range 1-9999."
            ))
        })
    }

    /// Parses like [`parse`](Self::parse); `None` on failure
    /// (.NET `DateTimeOffset.TryParse`).
    pub fn try_parse(text: &str, provider: &(impl DateTimeFormatProvider + ?Sized)) -> Option<DateTimeOffset> {
        Self::parse(text, provider).ok()
    }
}

impl Default for DateTimeOffset {
    /// [`DateTimeOffset::MIN_VALUE`], the C# `default(DateTimeOffset)`.
    fn default() -> Self {
        DateTimeOffset::MIN_VALUE
    }
}

impl From<DateTime> for DateTimeOffset {
    /// The C# implicit conversion; see [`DateTimeOffset::from_date_time`].
    #[track_caller]
    fn from(value: DateTime) -> Self {
        DateTimeOffset::from_date_time(value)
    }
}

impl PartialEq for DateTimeOffset {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.utc == other.utc
    }
}

impl Eq for DateTimeOffset {}

impl PartialOrd for DateTimeOffset {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DateTimeOffset {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.utc.cmp(&other.utc)
    }
}

impl Hash for DateTimeOffset {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.utc.hash(state);
    }
}

impl Add<TimeSpan> for DateTimeOffset {
    type Output = DateTimeOffset;
    #[track_caller]
    fn add(self, rhs: TimeSpan) -> DateTimeOffset {
        DateTimeOffset::add(self, rhs)
    }
}

impl Sub<TimeSpan> for DateTimeOffset {
    type Output = DateTimeOffset;
    #[track_caller]
    fn sub(self, rhs: TimeSpan) -> DateTimeOffset {
        self.subtract(rhs)
    }
}

impl Sub<DateTimeOffset> for DateTimeOffset {
    type Output = TimeSpan;
    #[inline]
    fn sub(self, rhs: DateTimeOffset) -> TimeSpan {
        self.subtract_date_time_offset(rhs)
    }
}

impl AddAssign<TimeSpan> for DateTimeOffset {
    #[track_caller]
    fn add_assign(&mut self, rhs: TimeSpan) {
        *self = *self + rhs;
    }
}

impl SubAssign<TimeSpan> for DateTimeOffset {
    #[track_caller]
    fn sub_assign(&mut self, rhs: TimeSpan) {
        *self = *self - rhs;
    }
}

impl fmt::Display for DateTimeOffset {
    /// The short date, long time and offset of the current culture (.NET `ToString()`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_format("", &CultureInfo::current_culture()))
    }
}

impl fmt::Debug for DateTimeOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let clock = self.date_time();
        let (year, month, day, _) = clock.date_parts();
        let minutes = self.offset_minutes.unsigned_abs();
        write!(
            f,
            "DateTimeOffset({year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:07}{}{:02}:{:02})",
            clock.hour(),
            clock.minute(),
            clock.second(),
            clock.ticks() % TimeSpan::TICKS_PER_SECOND,
            if self.offset_minutes < 0 { '-' } else { '+' },
            minutes / 60,
            minutes % 60
        )
    }
}
