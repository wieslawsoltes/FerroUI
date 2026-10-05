//! The Gregorian calendar operations the controls use
//! (.NET `System.Globalization.GregorianCalendar`) and ISO 8601 week numbers
//! (.NET `System.Globalization.ISOWeek`).

use super::{CalendarWeekRule, DateTime, DayOfWeek};

const OUT_OF_RANGE: &str = "The result is out of the supported range for this calendar.";

/// The proleptic Gregorian calendar. It has no state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GregorianCalendar;

impl GregorianCalendar {
    /// Creates the calendar.
    pub const fn new() -> GregorianCalendar {
        GregorianCalendar
    }

    /// The earliest date the calendar supports.
    pub const fn min_supported_date_time(&self) -> DateTime {
        DateTime::MIN_VALUE
    }

    /// The latest date the calendar supports.
    pub const fn max_supported_date_time(&self) -> DateTime {
        DateTime::MAX_VALUE
    }

    /// `time` moved by a number of days; `None` when the result is out of
    /// range (.NET throws `ArgumentException`).
    pub fn try_add_days(&self, time: DateTime, days: i32) -> Option<DateTime> {
        time.try_add_days(days as f64)
    }

    /// `time` moved by a number of days. Panics when the result is out of range.
    #[track_caller]
    pub fn add_days(&self, time: DateTime, days: i32) -> DateTime {
        self.try_add_days(time, days).expect(OUT_OF_RANGE)
    }

    /// `time` moved by a number of months, keeping the day where the
    /// resulting month has it; `None` when `months` is not in
    /// -120000..=120000 or the result is out of range.
    pub fn try_add_months(&self, time: DateTime, months: i32) -> Option<DateTime> {
        time.try_add_months(months)
    }

    /// `time` moved by a number of months. Panics when out of range.
    #[track_caller]
    pub fn add_months(&self, time: DateTime, months: i32) -> DateTime {
        self.try_add_months(time, months).expect(OUT_OF_RANGE)
    }

    /// `time` moved by a number of years; `None` when the result is out of range.
    pub fn try_add_years(&self, time: DateTime, years: i32) -> Option<DateTime> {
        years.checked_mul(12).and_then(|months| time.try_add_months(months))
    }

    /// `time` moved by a number of years. Panics when out of range.
    #[track_caller]
    pub fn add_years(&self, time: DateTime, years: i32) -> DateTime {
        self.try_add_years(time, years).expect(OUT_OF_RANGE)
    }

    /// The day of the week of `time`.
    pub fn get_day_of_week(&self, time: DateTime) -> DayOfWeek {
        time.day_of_week()
    }

    /// The number of days of a month. Panics for an invalid year or month.
    #[track_caller]
    pub fn get_days_in_month(&self, year: i32, month: i32) -> i32 {
        DateTime::days_in_month(year, month)
    }

    /// The number of the week of the year that contains `time`, counted from
    /// one, for the given rule and first day of the week.
    ///
    /// As in .NET, the last days of December are never counted into the first
    /// week of the following year: they get the number 53 (or 52) even where
    /// ISO 8601 says 1. Use [`IsoWeek::get_week_of_year`] for ISO numbers.
    pub fn get_week_of_year(&self, time: DateTime, rule: CalendarWeekRule, first_day_of_week: DayOfWeek) -> i32 {
        match rule {
            CalendarWeekRule::FirstDay => {
                let day_of_year = time.day_of_year() - 1;
                let offset = (Self::day_of_week_of_january_first(time) - first_day_of_week as i32 + 14) % 7;
                (day_of_year + offset) / 7 + 1
            }
            CalendarWeekRule::FirstFullWeek => Self::week_of_year_with_minimum_days(time, first_day_of_week as i32, 7),
            CalendarWeekRule::FirstFourDayWeek => Self::week_of_year_with_minimum_days(time, first_day_of_week as i32, 4),
        }
    }

    /// The day of the week (as a number, possibly negative modulo seven) of
    /// January 1 of the year of `time`.
    fn day_of_week_of_january_first(time: DateTime) -> i32 {
        time.day_of_week() as i32 - (time.day_of_year() - 1) % 7
    }

    fn week_of_year_with_minimum_days(time: DateTime, first_day_of_week: i32, minimum_days: i32) -> i32 {
        let mut time = time;
        loop {
            let day_of_year = time.day_of_year() - 1;
            // Days of the year that precede its first week.
            let mut offset = (first_day_of_week - Self::day_of_week_of_january_first(time) + 14) % 7;
            if offset != 0 && offset >= minimum_days {
                offset -= 7;
            }
            let day = day_of_year - offset;
            if day >= 0 {
                return day / 7 + 1;
            }
            // The date belongs to the last week of the previous year.
            match time.try_add_days(-(day_of_year as f64 + 1.0)) {
                Some(previous) => time = previous,
                None => return Self::week_of_year_before_first_year(first_day_of_week, minimum_days),
            }
        }
    }

    /// The week number of the first days of year 1 when they belong to the
    /// last week of the (unrepresentable, 365 days long) year before it.
    fn week_of_year_before_first_year(first_day_of_week: i32, minimum_days: i32) -> i32 {
        // 0001-01-01 is a Monday; a year of 365 days before it starts one weekday earlier, on a Sunday.
        let january_first_of_previous_year = DayOfWeek::Monday as i32 - 1 - 364 % 7;
        let partial_week = (first_day_of_week - january_first_of_previous_year + 14) % 7;
        let mut day = 364 - partial_week;
        if partial_week >= minimum_days {
            day += 7;
        }
        day / 7 + 1
    }
}

/// ISO 8601 week numbering (.NET `System.Globalization.ISOWeek`).
pub struct IsoWeek;

impl IsoWeek {
    /// The ISO week number of `date`, 1..=53: weeks start on Monday and the
    /// first week of a year is the one that contains its first Thursday.
    pub fn get_week_of_year(date: DateTime) -> i32 {
        date.to_date().iso_week() as i32
    }
}
