//! Culture-specific date and time conventions: patterns, names and calendar
//! rules (.NET `System.Globalization.DateTimeFormatInfo`).
//!
//! Only the conventions of the invariant culture are built in. The data of
//! every other culture comes from the registered
//! [`ICultureDataProvider`](super::ICultureDataProvider); a culture without
//! data uses the data of its nearest parent that has some and finally the
//! invariant data (.NET has data for every culture of the operating system).
//! The calendar is always the Gregorian one.

use super::i_culture_data_provider::find_culture_data;
use super::{CultureInfo, DayOfWeek, GregorianCalendar};
use std::borrow::Cow;
use std::ops::Deref;
use std::rc::Rc;

/// The rule that determines the first week of a year
/// (.NET `System.Globalization.CalendarWeekRule`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CalendarWeekRule {
    /// The first week starts on the first day of the year.
    #[default]
    FirstDay = 0,
    /// The first week is the first one that lies completely in the year.
    FirstFullWeek = 1,
    /// The first week is the first one with four or more days in the year.
    FirstFourDayWeek = 2,
}

const DAY_NAMES: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const ABBREVIATED_DAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const SHORTEST_DAY_NAMES: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November",
    "December",
];
const ABBREVIATED_MONTH_NAMES: [&str; 12] =
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

thread_local! {
    static INVARIANT: Rc<DateTimeFormatInfo> = Rc::new(DateTimeFormatInfo::new());
}

/// The date and time conventions of a culture.
///
/// The instances handed out by [`CultureInfo::date_time_format`] are shared
/// and read-only; to change conventions clone the value, use the setters and
/// attach it to a culture with [`CultureInfo::with_date_time_format`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateTimeFormatInfo {
    short_date_pattern: String,
    long_date_pattern: String,
    short_time_pattern: String,
    long_time_pattern: String,
    full_date_time_pattern: Option<String>,
    month_day_pattern: String,
    year_month_pattern: String,
    am_designator: String,
    pm_designator: String,
    date_separator: String,
    time_separator: String,
    era_name: String,
    first_day_of_week: DayOfWeek,
    calendar_week_rule: CalendarWeekRule,
    day_names: [String; 7],
    abbreviated_day_names: [String; 7],
    shortest_day_names: [String; 7],
    month_names: [String; 13],
    abbreviated_month_names: [String; 13],
}

fn day_strings(names: &[&str; 7]) -> [String; 7] {
    std::array::from_fn(|index| names[index].to_owned())
}

fn month_strings(names: &[&str; 12]) -> [String; 13] {
    std::array::from_fn(|index| names.get(index).map_or_else(String::new, |name| (*name).to_owned()))
}

impl DateTimeFormatInfo {
    /// Conventions with the values of the invariant culture (C#
    /// `new DateTimeFormatInfo()`); change them with the setters.
    pub fn new() -> DateTimeFormatInfo {
        DateTimeFormatInfo {
            short_date_pattern: "MM/dd/yyyy".to_owned(),
            long_date_pattern: "dddd, dd MMMM yyyy".to_owned(),
            short_time_pattern: "HH:mm".to_owned(),
            long_time_pattern: "HH:mm:ss".to_owned(),
            full_date_time_pattern: None,
            month_day_pattern: "MMMM dd".to_owned(),
            year_month_pattern: "yyyy MMMM".to_owned(),
            am_designator: "AM".to_owned(),
            pm_designator: "PM".to_owned(),
            date_separator: "/".to_owned(),
            time_separator: ":".to_owned(),
            era_name: "A.D.".to_owned(),
            first_day_of_week: DayOfWeek::Sunday,
            calendar_week_rule: CalendarWeekRule::FirstDay,
            day_names: day_strings(&DAY_NAMES),
            abbreviated_day_names: day_strings(&ABBREVIATED_DAY_NAMES),
            shortest_day_names: day_strings(&SHORTEST_DAY_NAMES),
            month_names: month_strings(&MONTH_NAMES),
            abbreviated_month_names: month_strings(&ABBREVIATED_MONTH_NAMES),
        }
    }

    /// The conventions of a culture without conventions of its own attached:
    /// the data the registered provider has for the culture or the nearest
    /// of its parents, else the invariant data.
    pub(crate) fn for_culture(culture: &CultureInfo) -> Rc<DateTimeFormatInfo> {
        find_culture_data(culture, |provider, name| provider.get_date_time_format(name)).unwrap_or_else(Self::invariant_info)
    }

    /// The conventions of the invariant culture.
    pub fn invariant_info() -> Rc<DateTimeFormatInfo> {
        INVARIANT.with(Rc::clone)
    }

    /// The conventions of the current culture.
    pub fn current_info() -> Rc<DateTimeFormatInfo> {
        CultureInfo::current_culture().date_time_format()
    }

    /// The calendar of the conventions; always the Gregorian one.
    pub fn calendar(&self) -> GregorianCalendar {
        GregorianCalendar
    }

    /// The pattern of a short date (format `"d"`).
    pub fn short_date_pattern(&self) -> &str {
        &self.short_date_pattern
    }

    /// Sets the pattern of a short date.
    pub fn set_short_date_pattern(&mut self, value: impl Into<String>) {
        self.short_date_pattern = value.into();
    }

    /// The pattern of a long date (format `"D"`).
    pub fn long_date_pattern(&self) -> &str {
        &self.long_date_pattern
    }

    /// Sets the pattern of a long date.
    pub fn set_long_date_pattern(&mut self, value: impl Into<String>) {
        self.long_date_pattern = value.into();
    }

    /// The pattern of a short time (format `"t"`).
    pub fn short_time_pattern(&self) -> &str {
        &self.short_time_pattern
    }

    /// Sets the pattern of a short time.
    pub fn set_short_time_pattern(&mut self, value: impl Into<String>) {
        self.short_time_pattern = value.into();
    }

    /// The pattern of a long time (format `"T"`).
    pub fn long_time_pattern(&self) -> &str {
        &self.long_time_pattern
    }

    /// Sets the pattern of a long time.
    pub fn set_long_time_pattern(&mut self, value: impl Into<String>) {
        self.long_time_pattern = value.into();
    }

    /// The pattern of a long date and long time (format `"F"`): the long
    /// date pattern, a space and the long time pattern unless set explicitly.
    pub fn full_date_time_pattern(&self) -> Cow<'_, str> {
        match &self.full_date_time_pattern {
            Some(pattern) => Cow::Borrowed(pattern),
            None => Cow::Owned(format!("{} {}", self.long_date_pattern, self.long_time_pattern)),
        }
    }

    /// Sets the pattern of a long date and long time.
    pub fn set_full_date_time_pattern(&mut self, value: impl Into<String>) {
        self.full_date_time_pattern = Some(value.into());
    }

    /// The pattern of a month and day (format `"M"`).
    pub fn month_day_pattern(&self) -> &str {
        &self.month_day_pattern
    }

    /// Sets the pattern of a month and day.
    pub fn set_month_day_pattern(&mut self, value: impl Into<String>) {
        self.month_day_pattern = value.into();
    }

    /// The pattern of a year and month (format `"Y"`).
    pub fn year_month_pattern(&self) -> &str {
        &self.year_month_pattern
    }

    /// Sets the pattern of a year and month.
    pub fn set_year_month_pattern(&mut self, value: impl Into<String>) {
        self.year_month_pattern = value.into();
    }

    /// The pattern of the round-trip format `"R"` (RFC 1123).
    pub fn rfc1123_pattern(&self) -> &'static str {
        "ddd, dd MMM yyyy HH':'mm':'ss 'GMT'"
    }

    /// The pattern of the sortable format `"s"`.
    pub fn sortable_date_time_pattern(&self) -> &'static str {
        "yyyy'-'MM'-'dd'T'HH':'mm':'ss"
    }

    /// The pattern of the universal sortable format `"u"`.
    pub fn universal_sortable_date_time_pattern(&self) -> &'static str {
        "yyyy'-'MM'-'dd HH':'mm':'ss'Z'"
    }

    /// The designator of hours before noon.
    pub fn am_designator(&self) -> &str {
        &self.am_designator
    }

    /// Sets the designator of hours before noon.
    pub fn set_am_designator(&mut self, value: impl Into<String>) {
        self.am_designator = value.into();
    }

    /// The designator of hours after noon.
    pub fn pm_designator(&self) -> &str {
        &self.pm_designator
    }

    /// Sets the designator of hours after noon.
    pub fn set_pm_designator(&mut self, value: impl Into<String>) {
        self.pm_designator = value.into();
    }

    /// The text the `/` specifier of a custom format produces.
    pub fn date_separator(&self) -> &str {
        &self.date_separator
    }

    /// Sets the date separator.
    pub fn set_date_separator(&mut self, value: impl Into<String>) {
        self.date_separator = value.into();
    }

    /// The text the `:` specifier of a custom format produces.
    pub fn time_separator(&self) -> &str {
        &self.time_separator
    }

    /// Sets the time separator.
    pub fn set_time_separator(&mut self, value: impl Into<String>) {
        self.time_separator = value.into();
    }

    /// The first day of the week.
    pub fn first_day_of_week(&self) -> DayOfWeek {
        self.first_day_of_week
    }

    /// Sets the first day of the week.
    pub fn set_first_day_of_week(&mut self, value: DayOfWeek) {
        self.first_day_of_week = value;
    }

    /// The rule that determines the first week of a year.
    pub fn calendar_week_rule(&self) -> CalendarWeekRule {
        self.calendar_week_rule
    }

    /// Sets the rule that determines the first week of a year.
    pub fn set_calendar_week_rule(&mut self, value: CalendarWeekRule) {
        self.calendar_week_rule = value;
    }

    /// The full names of the days, indexed by [`DayOfWeek`] (Sunday first).
    pub fn day_names(&self) -> &[String; 7] {
        &self.day_names
    }

    /// Sets the full names of the days.
    pub fn set_day_names(&mut self, value: [String; 7]) {
        self.day_names = value;
    }

    /// The abbreviated names of the days, indexed by [`DayOfWeek`].
    pub fn abbreviated_day_names(&self) -> &[String; 7] {
        &self.abbreviated_day_names
    }

    /// Sets the abbreviated names of the days.
    pub fn set_abbreviated_day_names(&mut self, value: [String; 7]) {
        self.abbreviated_day_names = value;
    }

    /// The shortest unique names of the days, indexed by [`DayOfWeek`].
    pub fn shortest_day_names(&self) -> &[String; 7] {
        &self.shortest_day_names
    }

    /// Sets the shortest names of the days.
    pub fn set_shortest_day_names(&mut self, value: [String; 7]) {
        self.shortest_day_names = value;
    }

    /// The full names of the months; the thirteenth entry is empty, as in .NET.
    pub fn month_names(&self) -> &[String; 13] {
        &self.month_names
    }

    /// Sets the full names of the months.
    pub fn set_month_names(&mut self, value: [String; 13]) {
        self.month_names = value;
    }

    /// The abbreviated names of the months; the thirteenth entry is empty.
    pub fn abbreviated_month_names(&self) -> &[String; 13] {
        &self.abbreviated_month_names
    }

    /// Sets the abbreviated names of the months.
    pub fn set_abbreviated_month_names(&mut self, value: [String; 13]) {
        self.abbreviated_month_names = value;
    }

    /// The full name of a day.
    pub fn get_day_name(&self, day_of_week: DayOfWeek) -> &str {
        &self.day_names[day_of_week as usize]
    }

    /// The abbreviated name of a day.
    pub fn get_abbreviated_day_name(&self, day_of_week: DayOfWeek) -> &str {
        &self.abbreviated_day_names[day_of_week as usize]
    }

    /// The shortest name of a day.
    pub fn get_shortest_day_name(&self, day_of_week: DayOfWeek) -> &str {
        &self.shortest_day_names[day_of_week as usize]
    }

    /// The full name of a month, 1..=13. Panics for other values.
    #[track_caller]
    pub fn get_month_name(&self, month: i32) -> &str {
        assert!((1..=13).contains(&month), "Valid values are between 1 and 13, inclusive. (Parameter 'month')");
        &self.month_names[(month - 1) as usize]
    }

    /// The abbreviated name of a month, 1..=13. Panics for other values.
    #[track_caller]
    pub fn get_abbreviated_month_name(&self, month: i32) -> &str {
        assert!((1..=13).contains(&month), "Valid values are between 1 and 13, inclusive. (Parameter 'month')");
        &self.abbreviated_month_names[(month - 1) as usize]
    }

    /// The name of the only era of the Gregorian calendar (the `g` specifier).
    pub fn get_era_name(&self, _era: i32) -> &str {
        &self.era_name
    }

    /// Sets the name of the era.
    pub fn set_era_name(&mut self, value: impl Into<String>) {
        self.era_name = value.into();
    }
}

impl Default for DateTimeFormatInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// The conventions a [`DateTimeFormatProvider`] hands out: borrowed from the
/// provider or shared.
pub enum DateTimeFormatInfoRef<'a> {
    /// Conventions owned by the provider.
    Borrowed(&'a DateTimeFormatInfo),
    /// Shared conventions.
    Shared(Rc<DateTimeFormatInfo>),
}

impl Deref for DateTimeFormatInfoRef<'_> {
    type Target = DateTimeFormatInfo;
    fn deref(&self) -> &DateTimeFormatInfo {
        match self {
            DateTimeFormatInfoRef::Borrowed(info) => info,
            DateTimeFormatInfoRef::Shared(info) => info,
        }
    }
}

/// Something that supplies date and time conventions: a [`CultureInfo`] or a
/// [`DateTimeFormatInfo`] (the role of `IFormatProvider` in the .NET date and
/// time members).
pub trait DateTimeFormatProvider {
    /// The conventions to format and parse with.
    fn get_date_time_format(&self) -> DateTimeFormatInfoRef<'_>;
}

impl DateTimeFormatProvider for DateTimeFormatInfo {
    fn get_date_time_format(&self) -> DateTimeFormatInfoRef<'_> {
        DateTimeFormatInfoRef::Borrowed(self)
    }
}

impl DateTimeFormatProvider for Rc<DateTimeFormatInfo> {
    fn get_date_time_format(&self) -> DateTimeFormatInfoRef<'_> {
        DateTimeFormatInfoRef::Borrowed(self)
    }
}

impl DateTimeFormatProvider for CultureInfo {
    fn get_date_time_format(&self) -> DateTimeFormatInfoRef<'_> {
        DateTimeFormatInfoRef::Shared(self.date_time_format())
    }
}

impl<T: DateTimeFormatProvider + ?Sized> DateTimeFormatProvider for &T {
    fn get_date_time_format(&self) -> DateTimeFormatInfoRef<'_> {
        (**self).get_date_time_format()
    }
}
