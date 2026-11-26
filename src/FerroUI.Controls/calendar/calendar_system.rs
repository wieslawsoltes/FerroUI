//! The calendar systems of a [`Calendar`](super::Calendar): WinUI's
//! `CalendarView.CalendarIdentifier` (`Windows.Globalization.CalendarIdentifiers`).
//!
//! A [`CalendarSystem`] does the month and year arithmetic of the views of a calendar and writes
//! their names. The Gregorian system with the conventions of the current culture is the default:
//! it is the arithmetic and the names of [`DateTimeHelper`] as they are, with nothing allocated.
//! The other systems, and the names of an explicit `Language`, come from ICU's compiled calendar
//! and date formatting data with the feature `calendar-systems`; without that feature every
//! identifier is the Gregorian calendar (docs/porting/winui-controls.md, section 10.14).

use super::DateTimeHelper;
use ferroui_base::utilities::{DateTime, DateTimeFormatInfo, DayOfWeek, Locale};
#[cfg(feature = "calendar-systems")]
use std::rc::Rc;

/// The identifiers of the calendar systems a [`Calendar`](super::Calendar) displays
/// (`Windows.Globalization.CalendarIdentifiers`), the values of its `CalendarIdentifier`.
pub struct CalendarIdentifiers;

impl CalendarIdentifiers {
    /// The proleptic Gregorian calendar, the default.
    pub const GREGORIAN: &'static str = "GregorianCalendar";
    /// The Hebrew lunisolar calendar: twelve or thirteen months, years of the creation era.
    pub const HEBREW: &'static str = "HebrewCalendar";
    /// The tabular (civil, arithmetic) Hijri calendar.
    pub const HIJRI: &'static str = "HijriCalendar";
    /// The Gregorian months with the eras of the Japanese emperors.
    pub const JAPANESE: &'static str = "JapaneseCalendar";
    /// The Julian calendar.
    pub const JULIAN: &'static str = "JulianCalendar";
    /// The Gregorian months with the years of the Tangun era (the Gregorian year plus 2333).
    pub const KOREAN: &'static str = "KoreanCalendar";
    /// The Solar Hijri calendar.
    pub const PERSIAN: &'static str = "PersianCalendar";
    /// The Gregorian months with the years of the Republic of China (Minguo).
    pub const TAIWAN: &'static str = "TaiwanCalendar";
    /// The Gregorian months with the years of the Buddhist era.
    pub const THAI: &'static str = "ThaiCalendar";
    /// The Umm al-Qura Hijri calendar of Saudi Arabia.
    pub const UM_AL_QURA: &'static str = "UmAlQuraCalendar";

    /// Every identifier, in the order of the upstream gallery's list.
    pub const ALL: [&'static str; 10] = [
        Self::GREGORIAN,
        Self::HEBREW,
        Self::HIJRI,
        Self::JAPANESE,
        Self::JULIAN,
        Self::KOREAN,
        Self::PERSIAN,
        Self::TAIWAN,
        Self::THAI,
        Self::UM_AL_QURA,
    ];
}

/// The calendar arithmetic and names a calendar's views use. Cloning it is cheap.
#[derive(Clone, Default)]
pub(crate) struct CalendarSystem {
    #[cfg(feature = "calendar-systems")]
    icu: Option<Rc<icu::IcuCalendar>>,
}

/// Runs the ICU arm when the system is not the default one, else the Gregorian arm.
macro_rules! dispatch {
    ($self:ident, $icu:ident => $other:expr, $gregorian:expr) => {{
        #[cfg(feature = "calendar-systems")]
        if let Some($icu) = $self.icu.as_deref() {
            return $other;
        }
        $gregorian
    }};
}

impl CalendarSystem {
    /// The system of an identifier (an unknown one is the Gregorian calendar) whose names are
    /// those of `language`, or of the current culture when it is `None`.
    pub(crate) fn new(identifier: &str, language: Option<&Locale>) -> CalendarSystem {
        #[cfg(feature = "calendar-systems")]
        {
            if identifier != CalendarIdentifiers::GREGORIAN || language.is_some() {
                return CalendarSystem { icu: Some(Rc::new(icu::IcuCalendar::new(identifier, language))) };
            }
        }
        let _ = (identifier, language);
        CalendarSystem::default()
    }

    /// Whether this is the default system (the Gregorian calendar of the current culture).
    #[allow(dead_code)] // read by the tests and without the feature `calendar-systems`
    pub(crate) fn is_default(&self) -> bool {
        dispatch!(self, _icu => false, true)
    }

    /// The year of `date` (an extended year: one number line across the eras).
    pub(crate) fn year(&self, date: DateTime) -> i32 {
        dispatch!(self, icu => icu.fields(date).0, date.year())
    }

    /// The month of `date`, from 1 (an ordinal: a leap month counts).
    #[allow(dead_code)] // read by the tests
    pub(crate) fn month(&self, date: DateTime) -> i32 {
        dispatch!(self, icu => icu.fields(date).1, date.month())
    }

    /// The day of the month of `date`, from 1.
    #[allow(dead_code)] // read by the tests
    pub(crate) fn day(&self, date: DateTime) -> i32 {
        dispatch!(self, icu => icu.fields(date).2, date.day())
    }

    /// The number of months of the year of `date`.
    pub(crate) fn months_in_year(&self, date: DateTime) -> i32 {
        dispatch!(self, icu => icu.months_in_year(date), 12)
    }

    /// The first day of the month of `date`, at midnight.
    pub(crate) fn discard_day_time(&self, date: DateTime) -> DateTime {
        dispatch!(self, icu => icu.first_of_month(date), DateTimeHelper::discard_day_time(date))
    }

    /// The first day of the year of `date`, at midnight.
    pub(crate) fn first_of_year(&self, date: DateTime) -> DateTime {
        dispatch!(
            self,
            icu => icu.date(icu.fields(date).0, 1, 1).unwrap_or_else(|| icu.first_of_month(date)),
            DateTime::new(date.year(), 1, 1)
        )
    }

    /// The first day of the last month of the year of `date`, at midnight.
    pub(crate) fn last_month_of_year(&self, date: DateTime) -> DateTime {
        dispatch!(
            self,
            icu => {
                let year = icu.fields(date).0;
                icu.date(year, icu.months_in_year(date), 1).unwrap_or_else(|| icu.first_of_month(date))
            },
            DateTime::new(date.year(), 12, 1)
        )
    }

    /// The first day of a year (as [`year`](Self::year) numbers it), or the nearest representable
    /// day of that year.
    pub(crate) fn year_start(&self, year: i32) -> DateTime {
        dispatch!(
            self,
            icu => icu.date(year, 1, 1).unwrap_or_else(|| {
                if year <= icu.fields(DateTime::MIN_VALUE).0 { DateTime::MIN_VALUE } else { DateTime::MAX_VALUE.date() }
            }),
            DateTime::new(year, 1, 1)
        )
    }

    /// `date` moved by a number of months, `None` when the result is not a representable date.
    pub(crate) fn add_months(&self, date: DateTime, months: i32) -> Option<DateTime> {
        dispatch!(self, icu => icu.add_months(date, months), DateTimeHelper::add_months(date, months))
    }

    /// `date` moved by a number of years, `None` when the result is not a representable date.
    pub(crate) fn add_years(&self, date: DateTime, years: i32) -> Option<DateTime> {
        dispatch!(self, icu => icu.add_years(date, years), DateTimeHelper::add_years(date, years))
    }

    /// The number of months `dt1` is after `dt2`.
    pub(crate) fn compare_year_month(&self, dt1: DateTime, dt2: DateTime) -> i32 {
        dispatch!(self, icu => icu.compare_year_month(dt1, dt2), DateTimeHelper::compare_year_month(dt1, dt2))
    }

    /// The first year of the decade of a date.
    pub(crate) fn decade_of_date(&self, date: DateTime) -> i32 {
        let year = self.year(date);
        year - year.rem_euclid(10)
    }

    /// The last year of the decade of a date.
    pub(crate) fn end_of_decade(&self, date: DateTime) -> i32 {
        self.decade_of_date(date) + 9
    }

    /// The header of the month view: the month and the year of `date`.
    pub(crate) fn month_year_text(&self, date: DateTime, format: &DateTimeFormatInfo) -> String {
        dispatch!(self, icu => icu.month_year_text(date), date.to_string_format("Y", format))
    }

    /// The abbreviated name of the month of `date` (a button of the year view).
    pub(crate) fn month_abbreviation(&self, date: DateTime, format: &DateTimeFormatInfo) -> String {
        dispatch!(
            self,
            icu => icu.month_abbreviation(date),
            format.abbreviated_month_names()[(date.month() - 1) as usize].clone()
        )
    }

    /// The text of a year (as [`year`](Self::year) numbers it).
    pub(crate) fn year_text(&self, year: i32) -> String {
        dispatch!(self, icu => icu.year_text(year), DateTimeHelper::format_number(year))
    }

    /// The text of the day of the month of `date`.
    pub(crate) fn day_text(&self, date: DateTime) -> String {
        dispatch!(self, icu => icu.day_text(date), DateTimeHelper::format_number(date.day()))
    }

    /// The title of a day of the week in the month view; `None` keeps the shortest day name of
    /// the current culture.
    pub(crate) fn day_title(&self, day: DayOfWeek) -> Option<String> {
        dispatch!(self, icu => icu.day_title(day), {
            let _ = day;
            None
        })
    }
}

#[cfg(feature = "calendar-systems")]
mod icu {
    use super::CalendarIdentifiers;
    use crate::calendar::DateTimeHelper;
    use ferroui_base::animation::SignedDuration;
    use ferroui_base::utilities::{CultureInfo, DateTime, DayOfWeek, Locale};
    use icu_calendar::preferences::{CalendarAlgorithm, HijriCalendarAlgorithm};
    use icu_calendar::types::{Month, RataDie};
    use icu_calendar::{AnyCalendar, AnyCalendarKind, Date, Iso, Ref};
    use icu_datetime::{fieldsets, DateTimeFormatter, DateTimeFormatterPreferences};

    const TICKS_PER_DAY: i64 = SignedDuration::TICKS_PER_DAY;
    /// The Tangun era of the Korean calendar starts 2333 years before the common era.
    const KOREAN_YEAR_OFFSET: i32 = 2333;

    /// How the names of a system are written.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Names {
        /// By the formatters, in the calendar of the system.
        Own,
        /// The Julian calendar has the Gregorian month names, and ICU formats no Julian dates:
        /// the fields are written as those of a Gregorian date.
        Julian,
        /// The Gregorian names with the year of the Tangun era.
        Korean,
    }

    pub(crate) struct IcuCalendar {
        calendar: AnyCalendar,
        /// Whether the years have a leap month (the Hebrew calendar), which makes the number of
        /// months before a year a formula instead of twelve per year.
        leap_months: bool,
        names: Names,
        year_month: DateTimeFormatter<fieldsets::YM>,
        month: DateTimeFormatter<fieldsets::M>,
        year: DateTimeFormatter<fieldsets::Y>,
        day: DateTimeFormatter<fieldsets::D>,
        weekday: DateTimeFormatter<fieldsets::E>,
    }

    /// The culture name of the names when no `Language` is set.
    fn current_locale() -> Locale {
        let culture = CultureInfo::current_formatting();
        Locale::try_from_str(culture.name()).ok().filter(|locale| !locale.id.language.is_unknown()).unwrap_or(
            ferroui_base::utilities::locale!("en-US"),
        )
    }

    impl IcuCalendar {
        pub(crate) fn new(identifier: &str, language: Option<&Locale>) -> IcuCalendar {
            let (kind, algorithm, names) = match identifier {
                CalendarIdentifiers::HEBREW => (AnyCalendarKind::Hebrew, CalendarAlgorithm::Hebrew, Names::Own),
                CalendarIdentifiers::HIJRI => (
                    AnyCalendarKind::HijriTabularTypeIIFriday,
                    CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Civil)),
                    Names::Own,
                ),
                CalendarIdentifiers::JAPANESE => (AnyCalendarKind::Japanese, CalendarAlgorithm::Japanese, Names::Own),
                CalendarIdentifiers::JULIAN => (AnyCalendarKind::Julian, CalendarAlgorithm::Gregory, Names::Julian),
                CalendarIdentifiers::KOREAN => (AnyCalendarKind::Gregorian, CalendarAlgorithm::Gregory, Names::Korean),
                CalendarIdentifiers::PERSIAN => (AnyCalendarKind::Persian, CalendarAlgorithm::Persian, Names::Own),
                CalendarIdentifiers::TAIWAN => (AnyCalendarKind::Roc, CalendarAlgorithm::Roc, Names::Own),
                CalendarIdentifiers::THAI => (AnyCalendarKind::Buddhist, CalendarAlgorithm::Buddhist, Names::Own),
                CalendarIdentifiers::UM_AL_QURA => (
                    AnyCalendarKind::HijriUmmAlQura,
                    CalendarAlgorithm::Hijri(Some(HijriCalendarAlgorithm::Umalqura)),
                    Names::Own,
                ),
                _ => (AnyCalendarKind::Gregorian, CalendarAlgorithm::Gregory, Names::Own),
            };
            let locale = language.cloned().unwrap_or_else(current_locale);
            let mut preferences = DateTimeFormatterPreferences::from(locale);
            preferences.calendar_algorithm = Some(algorithm);
            // The compiled data has every calendar of every locale: building a formatter fails
            // only for a calendar ICU does not format, and those are mapped above.
            macro_rules! formatter {
                ($fields:expr) => {
                    DateTimeFormatter::try_new(preferences, $fields).expect("the compiled data formats the calendar")
                };
            }
            IcuCalendar {
                calendar: AnyCalendar::new(kind),
                leap_months: kind == AnyCalendarKind::Hebrew,
                names,
                year_month: formatter!(fieldsets::YM::long()),
                month: formatter!(fieldsets::M::medium()),
                year: formatter!(fieldsets::Y::long()),
                day: formatter!(fieldsets::D::medium()),
                weekday: formatter!(fieldsets::E::short()),
            }
        }

        fn to_date(&self, date: DateTime) -> Date<Ref<'_, AnyCalendar>> {
            let rata_die = date.ticks().div_euclid(TICKS_PER_DAY) + 1;
            Date::from_rata_die(RataDie::new(rata_die), Ref(&self.calendar))
        }

        /// The midnight of a day counted as a rata die, `None` outside the range of `DateTime`.
        fn from_rata_die(rata_die: i64) -> Option<DateTime> {
            let ticks = (rata_die - 1).checked_mul(TICKS_PER_DAY)?;
            DateTime::try_from_ticks(ticks, DateTime::MIN_VALUE.kind())
        }

        /// The (year, ordinal month, day) of a date.
        pub(crate) fn fields(&self, date: DateTime) -> (i32, i32, i32) {
            let date = self.to_date(date);
            (date.extended_year(), date.month().ordinal as i32, date.day_of_month().0 as i32)
        }

        pub(crate) fn months_in_year(&self, date: DateTime) -> i32 {
            self.to_date(date).months_in_year() as i32
        }

        /// The number of the months before a year: twelve per year, or the months of the
        /// 19-year cycle of the Hebrew calendar (seven of its years have thirteen months).
        fn months_before(&self, year: i32) -> i64 {
            let year = year as i64;
            if self.leap_months {
                (235 * year - 234).div_euclid(19)
            } else {
                12 * year
            }
        }

        /// The year and ordinal month of a month counted by [`months_before`](Self::months_before).
        fn from_month_index(&self, index: i64) -> (i32, i32) {
            if !self.leap_months {
                return (index.div_euclid(12) as i32, (index.rem_euclid(12) + 1) as i32);
            }
            let mut year = (19 * index).div_euclid(235) as i32 + 1;
            while self.months_before(year) > index {
                year -= 1;
            }
            while self.months_before(year + 1) <= index {
                year += 1;
            }
            (year, (index - self.months_before(year) + 1) as i32)
        }

        fn month_index(&self, date: DateTime) -> i64 {
            let (year, month, _) = self.fields(date);
            self.months_before(year) + month as i64 - 1
        }

        /// The midnight of a day given by its fields, `None` when the day is not in that month
        /// or not representable.
        pub(crate) fn date(&self, year: i32, month: i32, day: i32) -> Option<DateTime> {
            let first = Date::try_new(year.into(), Month::new(1), 1, Ref(&self.calendar)).ok()?;
            if month < 1 || month > first.months_in_year() as i32 {
                return None;
            }
            let mut rata_die = first.to_rata_die().to_i64_date();
            for _ in 1..month {
                rata_die += Date::from_rata_die(RataDie::new(rata_die), Ref(&self.calendar)).days_in_month() as i64;
            }
            let start = Date::from_rata_die(RataDie::new(rata_die), Ref(&self.calendar));
            if day < 1 || day > start.days_in_month() as i32 {
                return None;
            }
            Self::from_rata_die(rata_die + day as i64 - 1)
        }

        pub(crate) fn first_of_month(&self, date: DateTime) -> DateTime {
            let day = self.to_date(date).day_of_month().0 as i64;
            let rata_die = date.ticks().div_euclid(TICKS_PER_DAY) + 1 - (day - 1);
            Self::from_rata_die(rata_die).unwrap_or(DateTime::MIN_VALUE)
        }

        /// The day `day` (or the last day) of a month, at the time of day of `time`.
        fn day_of_month(&self, year: i32, month: i32, day: i32, time: DateTime) -> Option<DateTime> {
            let first = self.date(year, month, 1)?;
            let days = self.to_date(first).days_in_month() as i32;
            let date = self.date(year, month, day.min(days))?;
            date.try_add_ticks(time.ticks().rem_euclid(TICKS_PER_DAY))
        }

        pub(crate) fn add_months(&self, date: DateTime, months: i32) -> Option<DateTime> {
            let (_, _, day) = self.fields(date);
            let (year, month) = self.from_month_index(self.month_index(date) + months as i64);
            self.day_of_month(year, month, day, date)
        }

        pub(crate) fn add_years(&self, date: DateTime, years: i32) -> Option<DateTime> {
            let icu_date = self.to_date(date);
            let year = icu_date.extended_year().checked_add(years)?;
            let day = icu_date.day_of_month().0 as i32;
            if !self.leap_months {
                return self.day_of_month(year, icu_date.month().ordinal as i32, day, date);
            }
            // The same month by its code: Adar I (a leap month) is Adar in a common year.
            let info = icu_date.month();
            let first = Date::try_new(year.into(), info.to_input(), 1, Ref(&self.calendar))
                .or_else(|_| Date::try_new(year.into(), Month::new(info.month_number() + 1), 1, Ref(&self.calendar)))
                .ok()?;
            self.day_of_month(year, first.month().ordinal as i32, day, date)
        }

        pub(crate) fn compare_year_month(&self, dt1: DateTime, dt2: DateTime) -> i32 {
            (self.month_index(dt1) - self.month_index(dt2)) as i32
        }

        /// The ISO date whose names are written for `date`: the date itself, or for the Julian
        /// calendar the Gregorian date with its fields (on a day every month has).
        fn named(&self, date: DateTime, day: Option<u8>) -> Option<Date<Iso>> {
            match self.names {
                Names::Julian => {
                    let (year, month, own_day) = self.fields(date);
                    Date::try_new_iso(year, month as u8, day.unwrap_or(own_day.min(28) as u8)).ok()
                }
                Names::Own | Names::Korean => Some(self.to_date(date).to_iso()),
            }
        }

        pub(crate) fn month_year_text(&self, date: DateTime) -> String {
            let Some(named) = self.named(date, Some(1)) else {
                return DateTimeHelper::format_number(self.fields(date).0);
            };
            let text = self.year_month.format(&named).to_string();
            if self.names == Names::Korean {
                let year = self.fields(date).0;
                return text.replacen(&year.to_string(), &(year + KOREAN_YEAR_OFFSET).to_string(), 1);
            }
            text
        }

        pub(crate) fn month_abbreviation(&self, date: DateTime) -> String {
            match self.named(date, Some(1)) {
                Some(named) => self.month.format(&named).to_string(),
                None => DateTimeHelper::format_number(self.fields(date).1),
            }
        }

        pub(crate) fn year_text(&self, year: i32) -> String {
            if self.names == Names::Korean {
                return (year + KOREAN_YEAR_OFFSET).to_string();
            }
            let named = match self.names {
                Names::Julian => Date::try_new_iso(year, 1, 1).ok(),
                _ => self.date(year, 1, 1).and_then(|date| self.named(date, None)),
            };
            match named {
                Some(named) => self.year.format(&named).to_string(),
                None => year.to_string(),
            }
        }

        pub(crate) fn day_text(&self, date: DateTime) -> String {
            let named = match self.names {
                // A Gregorian January day with the same number writes the digits of the day.
                Names::Julian => Date::try_new_iso(2001, 1, self.fields(date).2 as u8).ok(),
                _ => self.named(date, None),
            };
            match named {
                Some(named) => self.day.format(&named).to_string(),
                None => DateTimeHelper::format_number(self.fields(date).2),
            }
        }

        pub(crate) fn day_title(&self, day: DayOfWeek) -> Option<String> {
            // 1 January 2023 is a Sunday.
            let named = Date::try_new_iso(2023, 1, 1 + day.to_i32() as u8).ok()?;
            Some(self.weekday.format(&named).to_string())
        }
    }
}
