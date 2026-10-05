// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use super::CalendarDateRange;
use ferroui_base::utilities::{
    CalendarWeekRule, CultureInfo, DateTime, DateTimeFormatInfo, DayOfWeek, GregorianCalendar, IsoWeek,
};
use std::rc::Rc;

/// Date arithmetic and formatting helpers of the calendar controls.
pub(crate) struct DateTimeHelper;

impl DateTimeHelper {
    /// `time` moved by a number of days, `None` when the result is not a
    /// representable date.
    pub(crate) fn add_days(time: DateTime, days: i32) -> Option<DateTime> {
        GregorianCalendar::new().try_add_days(time, days)
    }

    /// `time` moved by a number of months, `None` when the result is not a
    /// representable date.
    pub(crate) fn add_months(time: DateTime, months: i32) -> Option<DateTime> {
        GregorianCalendar::new().try_add_months(time, months)
    }

    /// `time` moved by a number of years, `None` when the result is not a
    /// representable date.
    pub(crate) fn add_years(time: DateTime, years: i32) -> Option<DateTime> {
        GregorianCalendar::new().try_add_years(time, years)
    }

    /// Compares the dates of two values, ignoring the times of day.
    pub(crate) fn compare_days(dt1: DateTime, dt2: DateTime) -> i32 {
        DateTime::compare(Self::discard_time(dt1), Self::discard_time(dt2))
    }

    /// The number of months `dt1` is after `dt2`.
    pub(crate) fn compare_year_month(dt1: DateTime, dt2: DateTime) -> i32 {
        (dt1.year() - dt2.year()) * 12 + (dt1.month() - dt2.month())
    }

    /// The first year of the decade of a date.
    pub(crate) fn decade_of_date(date: DateTime) -> i32 {
        date.year() - (date.year() % 10)
    }

    /// The first day of the month of a date, at midnight.
    pub(crate) fn discard_day_time(d: DateTime) -> DateTime {
        DateTime::new_with_time(d.year(), d.month(), 1, 0, 0, 0)
    }

    /// The date of a value, at midnight.
    pub(crate) fn discard_time(d: DateTime) -> DateTime {
        d.date()
    }

    /// The last year of the decade of a date.
    pub(crate) fn end_of_decade(date: DateTime) -> i32 {
        Self::decade_of_date(date) + 9
    }

    /// The date and time conventions of the current culture.
    ///
    /// The calendar of a culture is always the Gregorian one here, so the
    /// conventions of the current culture are returned as they are. (The
    /// reference looks for a Gregorian calendar among the optional
    /// calendars of a culture whose default calendar is another one, and
    /// falls back to the invariant conventions.)
    pub(crate) fn get_current_date_format() -> Rc<DateTimeFormatInfo> {
        CultureInfo::current_culture().date_time_format()
    }

    /// Whether the day of `date` is within `range`.
    pub(crate) fn in_range(date: DateTime, range: &CalendarDateRange) -> bool {
        debug_assert!(DateTime::compare(range.start(), range.end()) < 1, "The range should start before it ends!");

        Self::compare_days(date, range.start()) > -1 && Self::compare_days(date, range.end()) < 1
    }

    /// Gets a localized string for the specified date using the year month
    /// pattern of the current culture.
    #[allow(dead_code)] // AUTOMATION-SEAM: used by the automation peers of the calendar (automation pass)
    pub(crate) fn to_year_month_pattern_string(date: DateTime) -> String {
        let format = Self::get_current_date_format();
        date.to_string_format(format.year_month_pattern(), &format)
    }

    /// Gets a localized string for the year of the specified date.
    #[allow(dead_code)] // AUTOMATION-SEAM: used by the automation peers of the calendar (automation pass)
    pub(crate) fn to_year_string(date: DateTime) -> String {
        Self::format_number(date.year())
    }

    /// The text of a number of the calendar (a day, a year, a cell index).
    ///
    /// The reference formats these with the date and time conventions as
    /// the format provider; those supply no number conventions, so the
    /// general number format applies, which for a number that is not
    /// negative is its digits in every culture.
    pub(crate) fn format_number(value: i32) -> String {
        debug_assert!(value >= 0);
        value.to_string()
    }

    /// The number of the week of the year that contains `date`.
    pub(crate) fn get_week_of_year(
        date: DateTime,
        rule: CalendarWeekRule,
        first_day_of_week: DayOfWeek,
        calendar: &GregorianCalendar,
    ) -> i32 {
        // The week of the year of the calendar is 53 for late December
        // dates that ISO 8601 assigns to week 1 of the next year (e.g.
        // 2018-12-31).
        if rule == CalendarWeekRule::FirstFourDayWeek && first_day_of_week == DayOfWeek::Monday {
            return IsoWeek::get_week_of_year(date);
        }

        calendar.get_week_of_year(date, rule, first_day_of_week)
    }
}
