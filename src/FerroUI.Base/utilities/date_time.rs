//! An instant in time as a date and time of day (.NET `System.DateTime`).
//!
//! The value is a count of 100-nanosecond ticks since 0001-01-01T00:00:00 in
//! the proleptic Gregorian calendar, together with a [`DateTimeKind`]. The
//! range is 0001-01-01T00:00:00 ..= 9999-12-31T23:59:59.9999999. Operations
//! whose result would leave that range panic (.NET throws
//! `ArgumentOutOfRangeException`); the `try_` forms return `None` instead.
//!
//! The calendar arithmetic (dates from day numbers, month lengths, leap
//! years, days of the week) is the one of the `time` crate; the tick count,
//! the kind, the range and the rules of the `add_` members are the .NET ones.

use super::date_time_format::{self, Parsed};
use super::{
    CultureInfo, DateTimeFormatProvider, DateTimeKind, DateTimeStyles, DayOfWeek, FormatError, TimeZoneInfo,
};
use crate::animation::TimeSpan;
use crate::platform::{IRuntimePlatform, StandardRuntimePlatform};
use crate::{FerroLocator, LocatorExtensions};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::rc::Rc;
use std::str::FromStr;

/// Gives the current UTC time.
pub type UtcNowProvider = Rc<dyn Fn() -> DateTime>;

thread_local! {
    // CLOCK-SEAM: tests pin "now" here. The wall clock itself is `IRuntimePlatform::get_utc_now`.
    static UTC_NOW_PROVIDER: RefCell<Option<UtcNowProvider>> = const { RefCell::new(None) };
}

const TICKS_PER_DAY: i64 = TimeSpan::TICKS_PER_DAY;
/// The Julian day number of 0001-01-01 (proleptic Gregorian).
const JULIAN_DAY_OF_YEAR_ONE: i32 = 1_721_426;
/// Days from 0001-01-01 to 10000-01-01.
const DAYS_TO_10000: i64 = 3_652_059;
/// Days from 0001-01-01 to 1970-01-01.
const DAYS_TO_1970: i64 = 719_162;
const MAX_TICKS: i64 = DAYS_TO_10000 * TICKS_PER_DAY - 1;
const MAX_MONTHS: i32 = 120_000;

const OUT_OF_RANGE: &str = "The added or subtracted value results in an un-representable DateTime.";

/// A date and time of day with a resolution of 100 nanoseconds.
///
/// Equality, ordering and hashing consider the ticks only; the kind is ignored,
/// as in .NET.
#[derive(Clone, Copy)]
pub struct DateTime {
    ticks: i64,
    kind: DateTimeKind,
}

impl DateTime {
    /// The smallest value, 0001-01-01T00:00:00.
    pub const MIN_VALUE: DateTime = DateTime { ticks: 0, kind: DateTimeKind::Unspecified };
    /// The largest value, 9999-12-31T23:59:59.9999999.
    pub const MAX_VALUE: DateTime = DateTime { ticks: MAX_TICKS, kind: DateTimeKind::Unspecified };
    /// 1970-01-01T00:00:00 UTC.
    pub const UNIX_EPOCH: DateTime = DateTime { ticks: DAYS_TO_1970 * TICKS_PER_DAY, kind: DateTimeKind::Utc };

    /// The date `year`-`month`-`day` at midnight, of unspecified kind.
    /// Panics when the arguments do not name a date in 0001..=9999.
    #[track_caller]
    pub fn new(year: i32, month: i32, day: i32) -> DateTime {
        Self::try_new(year, month, day).expect("Year, Month, and Day parameters describe an un-representable DateTime.")
    }

    /// The given date and time of day, of unspecified kind. Panics when the
    /// arguments do not name a valid date and time.
    #[track_caller]
    pub fn new_with_time(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32) -> DateTime {
        Self::try_new_with_time(year, month, day, hour, minute, second)
            .expect("The parameters describe an un-representable DateTime.")
    }

    /// The given date and time of day with milliseconds, of unspecified kind.
    /// Panics when the arguments do not name a valid date and time.
    #[track_caller]
    pub fn new_with_millisecond(
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: i32,
        millisecond: i32,
    ) -> DateTime {
        assert!((0..1000).contains(&millisecond), "Valid values are between 0 and 999, inclusive. (Parameter 'millisecond')");
        let value = Self::new_with_time(year, month, day, hour, minute, second);
        DateTime { ticks: value.ticks + millisecond as i64 * TimeSpan::TICKS_PER_MILLISECOND, kind: value.kind }
    }

    /// The date `year`-`month`-`day` at midnight; `None` when the arguments
    /// do not name a date in 0001..=9999.
    pub fn try_new(year: i32, month: i32, day: i32) -> Option<DateTime> {
        if !(1..=9999).contains(&year) {
            return None;
        }
        let date = time::Date::from_calendar_date(year, month_of(month)?, u8::try_from(day).ok()?).ok()?;
        let days = (date.to_julian_day() - JULIAN_DAY_OF_YEAR_ONE) as i64;
        Some(DateTime { ticks: days * TICKS_PER_DAY, kind: DateTimeKind::Unspecified })
    }

    /// The given date and time of day; `None` when the arguments do not name
    /// a valid date and time.
    pub fn try_new_with_time(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32) -> Option<DateTime> {
        if !(0..24).contains(&hour) || !(0..60).contains(&minute) || !(0..60).contains(&second) {
            return None;
        }
        let date = Self::try_new(year, month, day)?;
        let time = hour as i64 * TimeSpan::TICKS_PER_HOUR
            + minute as i64 * TimeSpan::TICKS_PER_MINUTE
            + second as i64 * TimeSpan::TICKS_PER_SECOND;
        Some(DateTime { ticks: date.ticks + time, kind: DateTimeKind::Unspecified })
    }

    /// The value with the given number of ticks, of unspecified kind. Panics
    /// when `ticks` is outside the range of the type.
    #[track_caller]
    pub const fn from_ticks(ticks: i64) -> DateTime {
        Self::from_ticks_kind(ticks, DateTimeKind::Unspecified)
    }

    /// The value with the given number of ticks and kind. Panics when `ticks`
    /// is outside the range of the type.
    #[track_caller]
    pub const fn from_ticks_kind(ticks: i64, kind: DateTimeKind) -> DateTime {
        assert!(
            ticks >= 0 && ticks <= MAX_TICKS,
            "Ticks must be between DateTime.MinValue.Ticks and DateTime.MaxValue.Ticks."
        );
        DateTime { ticks, kind }
    }

    /// The value with the given number of ticks; `None` when out of range.
    pub const fn try_from_ticks(ticks: i64, kind: DateTimeKind) -> Option<DateTime> {
        if ticks >= 0 && ticks <= MAX_TICKS {
            Some(DateTime { ticks, kind })
        } else {
            None
        }
    }

    /// The same ticks with another kind (.NET `DateTime.SpecifyKind`).
    #[inline]
    pub const fn specify_kind(self, kind: DateTimeKind) -> DateTime {
        DateTime { ticks: self.ticks, kind }
    }

    // --- the clock ---

    /// Installs (or with `None` removes) the source of the current UTC time
    /// of the current thread and returns the previous one. Tests use it to
    /// pin [`now`](Self::now), [`utc_now`](Self::utc_now) and [`today`](Self::today).
    pub fn set_utc_now_provider(provider: Option<UtcNowProvider>) -> Option<UtcNowProvider> {
        UTC_NOW_PROVIDER.with(|current| current.replace(provider))
    }

    /// The current date and time in UTC (kind [`DateTimeKind::Utc`]): the
    /// time of the pinned source of this thread
    /// ([`set_utc_now_provider`](Self::set_utc_now_provider)) when there is
    /// one, else the wall clock of the registered
    /// [`IRuntimePlatform`], else the one of the standard runtime platform.
    pub fn utc_now() -> DateTime {
        let provider = UTC_NOW_PROVIDER.with(|current| current.borrow().clone());
        if let Some(provider) = provider {
            return provider().to_universal_time();
        }
        let now = match FerroLocator::current().get_service::<dyn IRuntimePlatform>() {
            Some(platform) => platform.get_utc_now(),
            None => StandardRuntimePlatform::new().get_utc_now(),
        };
        now.specify_kind(DateTimeKind::Utc)
    }

    /// The current date and time in local time (kind [`DateTimeKind::Local`]).
    pub fn now() -> DateTime {
        Self::utc_now().to_local_time()
    }

    /// The current local date at midnight (kind [`DateTimeKind::Local`]).
    pub fn today() -> DateTime {
        Self::now().date()
    }

    // --- components ---

    /// The number of ticks since 0001-01-01T00:00:00.
    #[inline]
    pub const fn ticks(self) -> i64 {
        self.ticks
    }

    /// The kind of time.
    #[inline]
    pub const fn kind(self) -> DateTimeKind {
        self.kind
    }

    /// The date at midnight, with the same kind.
    #[inline]
    pub const fn date(self) -> DateTime {
        DateTime { ticks: self.ticks - self.ticks % TICKS_PER_DAY, kind: self.kind }
    }

    /// The time elapsed since midnight.
    #[inline]
    pub const fn time_of_day(self) -> TimeSpan {
        TimeSpan::from_ticks(self.ticks % TICKS_PER_DAY)
    }

    /// The calendar date of the value.
    pub(crate) fn to_date(self) -> time::Date {
        time::Date::from_julian_day((self.ticks / TICKS_PER_DAY) as i32 + JULIAN_DAY_OF_YEAR_ONE)
            .expect("every DateTime is a date of the years 1 to 9999")
    }

    /// The year, month, day of the month and day of the year (the latter two 1-based).
    pub(crate) fn date_parts(self) -> (i32, i32, i32, i32) {
        let date = self.to_date();
        (date.year(), date.month() as u8 as i32, date.day() as i32, date.ordinal() as i32)
    }

    /// The year, 1..=9999.
    pub fn year(self) -> i32 {
        self.date_parts().0
    }

    /// The month, 1..=12.
    pub fn month(self) -> i32 {
        self.date_parts().1
    }

    /// The day of the month, 1..=31.
    pub fn day(self) -> i32 {
        self.date_parts().2
    }

    /// The day of the year, 1..=366.
    pub fn day_of_year(self) -> i32 {
        self.date_parts().3
    }

    /// The day of the week.
    pub fn day_of_week(self) -> DayOfWeek {
        DayOfWeek::ALL[self.to_date().weekday().number_days_from_sunday() as usize]
    }

    /// The hour, 0..=23.
    #[inline]
    pub const fn hour(self) -> i32 {
        (self.ticks / TimeSpan::TICKS_PER_HOUR % 24) as i32
    }

    /// The minute, 0..=59.
    #[inline]
    pub const fn minute(self) -> i32 {
        (self.ticks / TimeSpan::TICKS_PER_MINUTE % 60) as i32
    }

    /// The second, 0..=59.
    #[inline]
    pub const fn second(self) -> i32 {
        (self.ticks / TimeSpan::TICKS_PER_SECOND % 60) as i32
    }

    /// The millisecond, 0..=999.
    #[inline]
    pub const fn millisecond(self) -> i32 {
        (self.ticks / TimeSpan::TICKS_PER_MILLISECOND % 1000) as i32
    }

    // --- calendar facts ---

    /// Whether `year` (1..=9999) is a leap year. Panics for other years.
    #[track_caller]
    pub fn is_leap_year(year: i32) -> bool {
        assert!((1..=9999).contains(&year), "Year must be between 1 and 9999. (Parameter 'year')");
        time::util::is_leap_year(year)
    }

    /// The number of days of `month` in `year`. Panics when the month is not
    /// 1..=12 or the year not 1..=9999.
    #[track_caller]
    pub fn days_in_month(year: i32, month: i32) -> i32 {
        let month = month_of(month).expect("Month must be between one and twelve. (Parameter 'month')");
        assert!((1..=9999).contains(&year), "Year must be between 1 and 9999. (Parameter 'year')");
        time::util::days_in_month(month, year) as i32
    }

    // --- arithmetic ---

    /// The value `ticks` later; `None` when out of range.
    pub const fn try_add_ticks(self, ticks: i64) -> Option<DateTime> {
        match self.ticks.checked_add(ticks) {
            Some(ticks) => Self::try_from_ticks(ticks, self.kind),
            None => None,
        }
    }

    /// The value `ticks` later. Panics when out of range.
    #[track_caller]
    pub fn add_ticks(self, ticks: i64) -> DateTime {
        self.try_add_ticks(ticks).expect(OUT_OF_RANGE)
    }

    /// The value `value` later. Panics when out of range.
    #[track_caller]
    pub fn add(self, value: TimeSpan) -> DateTime {
        self.add_ticks(value.ticks())
    }

    /// The value `value` later; `None` when out of range.
    pub fn try_add(self, value: TimeSpan) -> Option<DateTime> {
        self.try_add_ticks(value.ticks())
    }

    /// The value `value` earlier. Panics when out of range.
    #[track_caller]
    pub fn subtract(self, value: TimeSpan) -> DateTime {
        value.ticks().checked_neg().and_then(|ticks| self.try_add_ticks(ticks)).expect(OUT_OF_RANGE)
    }

    /// The interval from `other` to this value.
    #[inline]
    pub fn subtract_date_time(self, other: DateTime) -> TimeSpan {
        TimeSpan::from_ticks(self.ticks - other.ticks)
    }

    fn try_add_units(self, value: f64, ticks_per_unit: i64) -> Option<DateTime> {
        if !value.is_finite() || value.abs() > (MAX_TICKS / ticks_per_unit) as f64 {
            return None;
        }
        let integral = value.trunc();
        let fractional = value - integral;
        let ticks = integral as i64 * ticks_per_unit + (fractional * ticks_per_unit as f64) as i64;
        self.try_add_ticks(ticks)
    }

    /// The value `value` (whole and fractional) days later; `None` when out of range.
    pub fn try_add_days(self, value: f64) -> Option<DateTime> {
        self.try_add_units(value, TICKS_PER_DAY)
    }

    /// The value `value` (whole and fractional) days later. Panics when out of range.
    #[track_caller]
    pub fn add_days(self, value: f64) -> DateTime {
        self.try_add_days(value).expect(OUT_OF_RANGE)
    }

    /// The value `months` months later (earlier when negative). The day is
    /// kept, or becomes the last day of the resulting month when that month
    /// is shorter. `None` when `months` is not in -120000..=120000 or the
    /// result is out of range.
    pub fn try_add_months(self, months: i32) -> Option<DateTime> {
        if !(-MAX_MONTHS..=MAX_MONTHS).contains(&months) {
            return None;
        }
        let (year, month, day, _) = self.date_parts();
        let total = year * 12 + (month - 1) + months;
        let (year, month) = (total.div_euclid(12), total.rem_euclid(12) + 1);
        if !(1..=9999).contains(&year) {
            return None;
        }
        let day = day.min(Self::days_in_month(year, month));
        let date = Self::try_new(year, month, day)?;
        Some(DateTime { ticks: date.ticks + self.ticks % TICKS_PER_DAY, kind: self.kind })
    }

    /// The value `months` months later; see [`try_add_months`](Self::try_add_months).
    /// Panics when `months` or the result is out of range.
    #[track_caller]
    pub fn add_months(self, months: i32) -> DateTime {
        self.try_add_months(months).expect(OUT_OF_RANGE)
    }

    /// The value `years` years later (earlier when negative); February 29
    /// becomes February 28 in a year that is not a leap year. `None` when
    /// `years` is not in -10000..=10000 or the result is out of range.
    pub fn try_add_years(self, years: i32) -> Option<DateTime> {
        if !(-10_000..=10_000).contains(&years) {
            return None;
        }
        self.try_add_months(years * 12)
    }

    /// The value `years` years later; see [`try_add_years`](Self::try_add_years).
    /// Panics when `years` or the result is out of range.
    #[track_caller]
    pub fn add_years(self, years: i32) -> DateTime {
        self.try_add_years(years).expect(OUT_OF_RANGE)
    }

    // --- comparison ---

    /// Compares two values by their ticks: negative, zero or positive
    /// (.NET `DateTime.Compare`, which returns -1, 0 or 1).
    #[inline]
    pub fn compare(t1: DateTime, t2: DateTime) -> i32 {
        match t1.ticks.cmp(&t2.ticks) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    /// Compares this value with `other`: -1, 0 or 1.
    #[inline]
    pub fn compare_to(self, other: DateTime) -> i32 {
        Self::compare(self, other)
    }

    // --- time zone conversion ---

    /// The value converted to UTC. A value of unspecified kind is taken as
    /// local time. A result outside the range is clamped to it.
    pub fn to_universal_time(self) -> DateTime {
        if self.kind == DateTimeKind::Utc {
            return self;
        }
        let offset = TimeZoneInfo::get_utc_offset_of_local_time(self);
        DateTime { ticks: (self.ticks - offset.ticks()).clamp(0, MAX_TICKS), kind: DateTimeKind::Utc }
    }

    /// The value converted to local time. A value of unspecified kind is
    /// taken as UTC. A result outside the range is clamped to it.
    pub fn to_local_time(self) -> DateTime {
        if self.kind == DateTimeKind::Local {
            return self;
        }
        let offset = TimeZoneInfo::get_local_utc_offset(self);
        DateTime { ticks: (self.ticks + offset.ticks()).clamp(0, MAX_TICKS), kind: DateTimeKind::Local }
    }

    // --- formatting ---

    /// Formats with a standard or custom .NET date and time format string
    /// and the conventions of `provider` (a [`CultureInfo`] or a
    /// [`DateTimeFormatInfo`](super::DateTimeFormatInfo)). An empty format is
    /// the general format `"G"`. Panics when the format string is invalid
    /// (.NET throws `FormatException`).
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
        date_time_format::format(self, None, format, &provider.get_date_time_format())
    }

    /// Formats with a format string and the conventions of the current
    /// culture (.NET `ToString(string)`). Panics when the format is invalid.
    #[track_caller]
    pub fn to_string_with(self, format: &str) -> String {
        self.to_string_format(format, &CultureInfo::current_culture())
    }

    /// Formats with the general format `"G"` and the conventions of
    /// `provider` (.NET `ToString(IFormatProvider)`).
    pub fn to_string_provider(self, provider: &(impl DateTimeFormatProvider + ?Sized)) -> String {
        self.to_string_format("G", provider)
    }

    // --- parsing ---

    fn from_parsed(parsed: Parsed, styles: DateTimeStyles) -> Result<DateTime, FormatError> {
        let clock = parsed.date_time;
        let out_of_range = || FormatError::new("The UTC representation of the date falls outside the supported range.");
        match parsed.offset {
            Some(offset) => {
                let utc = DateTime::try_from_ticks(clock.ticks - offset.ticks(), DateTimeKind::Utc).ok_or_else(out_of_range)?;
                if styles.contains(DateTimeStyles::ADJUST_TO_UNIVERSAL)
                    || (styles.contains(DateTimeStyles::ROUNDTRIP_KIND) && parsed.utc_designator)
                {
                    Ok(utc)
                } else {
                    Ok(utc.to_local_time())
                }
            }
            None if styles.contains(DateTimeStyles::ASSUME_UNIVERSAL) => {
                let utc = clock.specify_kind(DateTimeKind::Utc);
                Ok(if styles.contains(DateTimeStyles::ADJUST_TO_UNIVERSAL) { utc } else { utc.to_local_time() })
            }
            None if styles.contains(DateTimeStyles::ASSUME_LOCAL) => {
                let local = clock.specify_kind(DateTimeKind::Local);
                Ok(if styles.contains(DateTimeStyles::ADJUST_TO_UNIVERSAL) { local.to_universal_time() } else { local })
            }
            None => Ok(clock),
        }
    }

    /// Parses a date and time written in the conventions of `provider`
    /// (.NET `DateTime.Parse(string, IFormatProvider)`).
    ///
    /// This is not the heuristic parser of .NET. It is checked against
    /// results recorded from .NET (`date_time_net_data.rs`, tables `PARSE`,
    /// `PARSE_ROUND_TRIP` and `PARSE_STYLES`) for the invariant culture,
    /// `en`, `en-US` and `en-GB`, which covers what the date picker feeds
    /// it: the text it writes itself with the short and the long date
    /// pattern of the culture, and typed dates of these forms:
    ///
    /// * numeric dates with `/`, `-`, `.` or spaces in the order of the
    ///   short date pattern; a year of three or more digits written first
    ///   gives year-month-day; two-digit years are 1950..=2049;
    /// * month names and abbreviations (of the culture or English), with
    ///   the day before or after and an optional, matching day name;
    /// * a month and a year alone (the first of the month), a month and a
    ///   day alone (the current year);
    /// * an offset from UTC (`Z`, `GMT`, a signed `hh:mm`, `hhmm` or `h[h]`)
    ///   after the time or after a date without a time; the result is then
    ///   the local time of that instant;
    /// * ISO 8601 `yyyy-MM-ddTHH:mm:ss[.fffffff]` with `Z` or an offset,
    ///   the sortable, universal sortable and RFC 1123 formats;
    /// * a time `H:mm[:ss[.fffffff]]` with an optional designator before or
    ///   after the date, or alone (the current date).
    ///
    /// Everything else is rejected, and has not been compared with .NET
    /// beyond the recorded rows: other time zone names than `GMT`
    /// and `Z` (.NET rejects `UTC` too), era names, a two-number date that is only valid in the
    /// order the culture does not use, and the inflected (genitive) month
    /// names of cultures that have them, which .NET accepts.
    pub fn parse(text: &str, provider: &(impl DateTimeFormatProvider + ?Sized)) -> Result<DateTime, FormatError> {
        Self::parse_with_styles(text, provider, DateTimeStyles::NONE)
    }

    /// Parses a date and time written in the conventions of `provider`
    /// (.NET `DateTime.Parse(string, IFormatProvider, DateTimeStyles)`).
    pub fn parse_with_styles(
        text: &str,
        provider: &(impl DateTimeFormatProvider + ?Sized),
        styles: DateTimeStyles,
    ) -> Result<DateTime, FormatError> {
        let parsed = date_time_format::parse(text, &provider.get_date_time_format(), styles)?;
        Self::from_parsed(parsed, styles)
    }

    /// Parses like [`parse_with_styles`](Self::parse_with_styles); `None` on failure
    /// (.NET `DateTime.TryParse`).
    pub fn try_parse(
        text: &str,
        provider: &(impl DateTimeFormatProvider + ?Sized),
        styles: DateTimeStyles,
    ) -> Option<DateTime> {
        Self::parse_with_styles(text, provider, styles).ok()
    }

    /// Parses a date and time that has to match `format` exactly
    /// (.NET `DateTime.ParseExact(string, string, IFormatProvider)`).
    pub fn parse_exact(
        text: &str,
        format: &str,
        provider: &(impl DateTimeFormatProvider + ?Sized),
    ) -> Result<DateTime, FormatError> {
        Self::parse_exact_with_styles(text, format, provider, DateTimeStyles::NONE)
    }

    /// Parses a date and time that has to match `format` exactly
    /// (.NET `DateTime.ParseExact(string, string, IFormatProvider, DateTimeStyles)`).
    pub fn parse_exact_with_styles(
        text: &str,
        format: &str,
        provider: &(impl DateTimeFormatProvider + ?Sized),
        styles: DateTimeStyles,
    ) -> Result<DateTime, FormatError> {
        let parsed = date_time_format::parse_exact(text, format, &provider.get_date_time_format(), styles)?;
        Self::from_parsed(parsed, styles)
    }

    /// Parses a date and time that has to match one of `formats` exactly; the
    /// first format that matches wins (.NET `DateTime.ParseExact(string, string[], ..)`).
    pub fn parse_exact_multiple(
        text: &str,
        formats: &[&str],
        provider: &(impl DateTimeFormatProvider + ?Sized),
        styles: DateTimeStyles,
    ) -> Result<DateTime, FormatError> {
        let info = provider.get_date_time_format();
        for format in formats {
            if let Ok(parsed) = date_time_format::parse_exact(text, format, &info, styles) {
                if let Ok(value) = Self::from_parsed(parsed, styles) {
                    return Ok(value);
                }
            }
        }
        Err(FormatError::from_string(format!("String '{text}' was not recognized as a valid DateTime.")))
    }

    /// Like [`parse_exact_with_styles`](Self::parse_exact_with_styles); `None` on failure
    /// (.NET `DateTime.TryParseExact(string, string, ..)`).
    pub fn try_parse_exact(
        text: &str,
        format: &str,
        provider: &(impl DateTimeFormatProvider + ?Sized),
        styles: DateTimeStyles,
    ) -> Option<DateTime> {
        Self::parse_exact_with_styles(text, format, provider, styles).ok()
    }

    /// Like [`parse_exact_multiple`](Self::parse_exact_multiple); `None` on failure
    /// (.NET `DateTime.TryParseExact(string, string[], ..)`).
    pub fn try_parse_exact_multiple(
        text: &str,
        formats: &[&str],
        provider: &(impl DateTimeFormatProvider + ?Sized),
        styles: DateTimeStyles,
    ) -> Option<DateTime> {
        Self::parse_exact_multiple(text, formats, provider, styles).ok()
    }
}

/// The month of the `time` crate for a month number; `None` unless 1..=12.
fn month_of(month: i32) -> Option<time::Month> {
    time::Month::try_from(u8::try_from(month).ok()?).ok()
}

impl Default for DateTime {
    /// [`DateTime::MIN_VALUE`], the C# `default(DateTime)`.
    fn default() -> Self {
        DateTime::MIN_VALUE
    }
}

impl PartialEq for DateTime {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ticks == other.ticks
    }
}

impl Eq for DateTime {}

impl PartialOrd for DateTime {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DateTime {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.ticks.cmp(&other.ticks)
    }
}

impl Hash for DateTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ticks.hash(state);
    }
}

impl Add<TimeSpan> for DateTime {
    type Output = DateTime;
    #[track_caller]
    fn add(self, rhs: TimeSpan) -> DateTime {
        DateTime::add(self, rhs)
    }
}

impl Sub<TimeSpan> for DateTime {
    type Output = DateTime;
    #[track_caller]
    fn sub(self, rhs: TimeSpan) -> DateTime {
        self.subtract(rhs)
    }
}

impl Sub<DateTime> for DateTime {
    type Output = TimeSpan;
    #[inline]
    fn sub(self, rhs: DateTime) -> TimeSpan {
        self.subtract_date_time(rhs)
    }
}

impl AddAssign<TimeSpan> for DateTime {
    #[track_caller]
    fn add_assign(&mut self, rhs: TimeSpan) {
        *self = *self + rhs;
    }
}

impl SubAssign<TimeSpan> for DateTime {
    #[track_caller]
    fn sub_assign(&mut self, rhs: TimeSpan) {
        *self = *self - rhs;
    }
}

impl fmt::Display for DateTime {
    /// The general format `"G"` of the current culture (.NET `ToString()`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_format("G", &CultureInfo::current_culture()))
    }
}

impl fmt::Debug for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day, _) = self.date_parts();
        write!(
            f,
            "DateTime({year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:07}, {:?})",
            self.hour(),
            self.minute(),
            self.second(),
            self.ticks % TimeSpan::TICKS_PER_SECOND,
            self.kind
        )
    }
}

impl FromStr for DateTime {
    type Err = FormatError;
    /// Parses with the conventions of the current culture (.NET `DateTime.Parse(string)`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DateTime::parse(s, &CultureInfo::current_culture())
    }
}
