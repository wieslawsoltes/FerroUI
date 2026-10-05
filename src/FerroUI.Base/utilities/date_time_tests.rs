//! Tests of the date and time types. There are no upstream tests for these
//! (they are runtime types there); the expectations are the documented .NET
//! results.

use super::{
    CalendarWeekRule, CultureInfo, DateTime, DateTimeFormatInfo, DateTimeKind, DateTimeOffset, DateTimeStyles,
    DayOfWeek, GregorianCalendar, IsoWeek, TestCultureDataProvider, TimeZoneInfo,
};
use crate::animation::TimeSpan;
use std::rc::Rc;

fn invariant() -> CultureInfo {
    CultureInfo::invariant_culture()
}

/// A culture of the test culture data (`en-US`, `en-GB`); every other name
/// has no data and falls back.
fn culture_of(name: &str) -> CultureInfo {
    thread_local! {
        static REGISTERED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if !REGISTERED.replace(true) {
        TestCultureDataProvider::register();
    }
    CultureInfo::get_culture_info(name)
}

fn en_us() -> CultureInfo {
    culture_of("en-US")
}

/// A culture that writes dates day first with `.` as separator.
fn day_first() -> CultureInfo {
    let mut info = DateTimeFormatInfo::new();
    info.set_short_date_pattern("dd.MM.yyyy");
    info.set_long_date_pattern("dddd, d. MMMM yyyy");
    info.set_date_separator(".");
    CultureInfo::get_culture_info("x-day-first").with_date_time_format(info)
}

/// A culture that writes dates year first with unit characters.
fn year_first() -> CultureInfo {
    let mut info = DateTimeFormatInfo::new();
    info.set_short_date_pattern("yyyy/MM/dd");
    info.set_long_date_pattern("yyyy年M月d日");
    info.set_year_month_pattern("yyyy年M月");
    CultureInfo::get_culture_info("x-year-first").with_date_time_format(info)
}

fn pin_now(utc: DateTime, offset_hours: i64) {
    DateTime::set_utc_now_provider(Some(Rc::new(move || utc.specify_kind(DateTimeKind::Utc))));
    TimeZoneInfo::set_local_utc_offset_provider(Some(Rc::new(move |_| {
        TimeSpan::from_ticks(offset_hours * TimeSpan::TICKS_PER_HOUR)
    })));
}

fn ignore<T>(_: T) {}

fn panics(f: impl FnOnce() + std::panic::UnwindSafe) -> bool {
    std::panic::catch_unwind(f).is_err()
}

// --- DateTime: construction and components ---

#[test]
fn min_and_max_values() {
    assert_eq!(DateTime::MIN_VALUE.ticks(), 0);
    assert_eq!(DateTime::MAX_VALUE.ticks(), 3_155_378_975_999_999_999);
    assert_eq!(DateTime::MIN_VALUE, DateTime::new(1, 1, 1));
    assert_eq!(DateTime::MAX_VALUE.date(), DateTime::new(9999, 12, 31));
    assert_eq!(DateTime::default(), DateTime::MIN_VALUE);
    assert_eq!(DateTime::UNIX_EPOCH, DateTime::new(1970, 1, 1));
    assert_eq!(DateTime::UNIX_EPOCH.ticks(), 621_355_968_000_000_000);
}

#[test]
fn components_round_trip() {
    let value = DateTime::new_with_millisecond(2024, 2, 29, 13, 45, 59, 123);
    assert_eq!((value.year(), value.month(), value.day()), (2024, 2, 29));
    assert_eq!((value.hour(), value.minute(), value.second(), value.millisecond()), (13, 45, 59, 123));
    assert_eq!(value.day_of_year(), 60);
    assert_eq!(value.day_of_week(), DayOfWeek::Thursday);
    assert_eq!(value.date(), DateTime::new(2024, 2, 29));
    assert_eq!(value.time_of_day(), TimeSpan::from_dhms_milliseconds(0, 13, 45, 59, 123));
    assert_eq!(value.kind(), DateTimeKind::Unspecified);
    assert_eq!(DateTime::new(2000, 1, 1).ticks(), 630_822_816_000_000_000);
    assert_eq!(DateTime::new(2000, 10, 10).ticks(), 631_067_328_000_000_000);
}

#[test]
fn every_day_of_some_years_round_trips() {
    for year in [1, 4, 100, 400, 1900, 1999, 2000, 2023, 2024, 9999] {
        let mut expected_day_of_year = 0;
        for month in 1..=12 {
            for day in 1..=DateTime::days_in_month(year, month) {
                expected_day_of_year += 1;
                let value = DateTime::new(year, month, day);
                assert_eq!((value.year(), value.month(), value.day()), (year, month, day));
                assert_eq!(value.day_of_year(), expected_day_of_year);
            }
        }
        assert_eq!(expected_day_of_year, if DateTime::is_leap_year(year) { 366 } else { 365 });
    }
}

#[test]
fn day_of_week_of_known_dates() {
    assert_eq!(DateTime::new(1, 1, 1).day_of_week(), DayOfWeek::Monday);
    assert_eq!(DateTime::new(1970, 1, 1).day_of_week(), DayOfWeek::Thursday);
    assert_eq!(DateTime::new(2000, 1, 1).day_of_week(), DayOfWeek::Saturday);
    assert_eq!(DateTime::new(2021, 1, 4).day_of_week(), DayOfWeek::Monday);
    assert_eq!(DateTime::new(9999, 12, 31).day_of_week(), DayOfWeek::Friday);
}

#[test]
fn leap_years_and_month_lengths() {
    assert!(DateTime::is_leap_year(2000));
    assert!(DateTime::is_leap_year(2024));
    assert!(!DateTime::is_leap_year(1900));
    assert!(!DateTime::is_leap_year(2023));
    assert_eq!(DateTime::days_in_month(2024, 2), 29);
    assert_eq!(DateTime::days_in_month(2023, 2), 28);
    assert_eq!(DateTime::days_in_month(2023, 12), 31);
    assert_eq!(DateTime::days_in_month(2023, 4), 30);
    assert!(panics(|| ignore(DateTime::days_in_month(2023, 13))));
    assert!(panics(|| ignore(DateTime::is_leap_year(0))));
}

#[test]
fn invalid_dates_are_rejected() {
    assert!(DateTime::try_new(2023, 2, 29).is_none());
    assert!(DateTime::try_new(0, 1, 1).is_none());
    assert!(DateTime::try_new(10000, 1, 1).is_none());
    assert!(DateTime::try_new(2023, 13, 1).is_none());
    assert!(DateTime::try_new(2023, 1, 0).is_none());
    assert!(DateTime::try_new_with_time(2023, 1, 1, 24, 0, 0).is_none());
    assert!(DateTime::try_new_with_time(2023, 1, 1, 0, 60, 0).is_none());
    assert!(panics(|| ignore(DateTime::new(2023, 2, 30))));
    assert!(panics(|| ignore(DateTime::from_ticks(-1))));
    assert!(panics(|| ignore(DateTime::from_ticks(DateTime::MAX_VALUE.ticks() + 1))));
}

// --- DateTime: arithmetic ---

#[test]
fn add_days_and_overflow() {
    assert_eq!(DateTime::new(2023, 12, 31).add_days(1.0), DateTime::new(2024, 1, 1));
    assert_eq!(DateTime::new(2024, 3, 1).add_days(-1.0), DateTime::new(2024, 2, 29));
    assert_eq!(DateTime::new(2024, 1, 1).add_days(0.5), DateTime::new_with_time(2024, 1, 1, 12, 0, 0));
    assert_eq!(DateTime::MAX_VALUE.add_days(-1.0).date(), DateTime::new(9999, 12, 30));
    assert!(DateTime::MAX_VALUE.try_add_days(1.0).is_none());
    assert!(DateTime::MIN_VALUE.try_add_days(-1.0).is_none());
    assert!(panics(|| ignore(DateTime::MAX_VALUE.add_days(1.0))));
    assert!(panics(|| ignore(DateTime::MIN_VALUE.add_ticks(-1))));
}

#[test]
fn add_months_clamps_the_day() {
    assert_eq!(DateTime::new(2024, 1, 31).add_months(1), DateTime::new(2024, 2, 29));
    assert_eq!(DateTime::new(2023, 1, 31).add_months(1), DateTime::new(2023, 2, 28));
    assert_eq!(DateTime::new(2023, 3, 31).add_months(-1), DateTime::new(2023, 2, 28));
    assert_eq!(DateTime::new(2023, 11, 30).add_months(3), DateTime::new(2024, 2, 29));
    assert_eq!(DateTime::new(2023, 1, 15).add_months(-13), DateTime::new(2021, 12, 15));
    assert_eq!(DateTime::new(2023, 12, 15).add_months(1), DateTime::new(2024, 1, 15));
    let with_time = DateTime::new_with_time(2023, 5, 31, 10, 20, 30).add_months(1);
    assert_eq!(with_time, DateTime::new_with_time(2023, 6, 30, 10, 20, 30));
    assert!(DateTime::new(9999, 12, 1).try_add_months(1).is_none());
    assert!(DateTime::new(1, 1, 1).try_add_months(-1).is_none());
    assert!(DateTime::new(2000, 1, 1).try_add_months(120_001).is_none());
    assert!(panics(|| ignore(DateTime::new(9999, 12, 1).add_months(1))));
}

#[test]
fn add_years_handles_leap_days() {
    assert_eq!(DateTime::new(2024, 2, 29).add_years(1), DateTime::new(2025, 2, 28));
    assert_eq!(DateTime::new(2024, 2, 29).add_years(4), DateTime::new(2028, 2, 29));
    assert_eq!(DateTime::new(2024, 2, 29).add_years(-24), DateTime::new(2000, 2, 29));
    assert!(DateTime::new(9999, 1, 1).try_add_years(1).is_none());
    assert!(DateTime::new(1, 1, 1).try_add_years(-1).is_none());
}

#[test]
fn operators_with_time_span() {
    let start = DateTime::new(2024, 1, 1);
    let end = start + TimeSpan::from_hours(36.0);
    assert_eq!(end, DateTime::new_with_time(2024, 1, 2, 12, 0, 0));
    assert_eq!(end - start, TimeSpan::from_hours(36.0));
    assert_eq!(start - end, TimeSpan::from_hours(-36.0));
    assert_eq!(end - TimeSpan::from_hours(36.0), start);
    let mut value = start;
    value += TimeSpan::from_days(1.0);
    value -= TimeSpan::from_hours(1.0);
    assert_eq!(value, DateTime::new_with_time(2024, 1, 1, 23, 0, 0));
}

#[test]
fn comparisons_ignore_the_kind() {
    let utc = DateTime::new(2024, 1, 1).specify_kind(DateTimeKind::Utc);
    let local = DateTime::new(2024, 1, 1).specify_kind(DateTimeKind::Local);
    assert_eq!(utc, local);
    assert_eq!(DateTime::compare(utc, local), 0);
    assert_eq!(DateTime::compare(DateTime::new(2024, 1, 1), DateTime::new(2024, 1, 2)), -1);
    assert_eq!(DateTime::new(2024, 1, 2).compare_to(DateTime::new(2024, 1, 1)), 1);
    assert!(DateTime::new(2024, 1, 1) < DateTime::new(2024, 1, 2));
    assert_eq!(DateTime::new(2024, 1, 1).max(DateTime::new(2024, 1, 2)), DateTime::new(2024, 1, 2));
}

// --- the clock and the time zone ---

#[test]
fn now_can_be_pinned() {
    pin_now(DateTime::new_with_time(2024, 10, 12, 23, 30, 0), 2);
    assert_eq!(DateTime::utc_now(), DateTime::new_with_time(2024, 10, 12, 23, 30, 0));
    assert_eq!(DateTime::utc_now().kind(), DateTimeKind::Utc);
    assert_eq!(DateTime::now(), DateTime::new_with_time(2024, 10, 13, 1, 30, 0));
    assert_eq!(DateTime::now().kind(), DateTimeKind::Local);
    assert_eq!(DateTime::today(), DateTime::new(2024, 10, 13));
    assert_eq!(DateTimeOffset::now().offset(), TimeSpan::from_hours(2.0));
    assert_eq!(DateTimeOffset::now().date_time(), DateTime::new_with_time(2024, 10, 13, 1, 30, 0));
    assert_eq!(DateTimeOffset::utc_now().offset(), TimeSpan::ZERO);

    DateTime::set_utc_now_provider(None);
    TimeZoneInfo::set_local_utc_offset_provider(None);
    assert!(DateTime::utc_now() > DateTime::new(2024, 1, 1));
    assert_eq!(DateTimeOffset::now().offset(), TimeSpan::ZERO);
}

#[test]
fn local_and_universal_conversion() {
    pin_now(DateTime::new(2024, 1, 1), -5);
    let local = DateTime::new_with_time(2024, 6, 1, 10, 0, 0);
    let utc = local.to_universal_time();
    assert_eq!(utc, DateTime::new_with_time(2024, 6, 1, 15, 0, 0));
    assert_eq!(utc.kind(), DateTimeKind::Utc);
    assert_eq!(utc.to_universal_time(), utc);
    assert_eq!(utc.to_local_time(), local);
    assert_eq!(utc.to_local_time().kind(), DateTimeKind::Local);
    assert_eq!(DateTime::MIN_VALUE.specify_kind(DateTimeKind::Utc).to_local_time(), DateTime::MIN_VALUE);
}

// --- formatting ---

#[test]
fn standard_formats_of_the_invariant_culture() {
    let value = DateTime::new_with_time(2024, 10, 5, 14, 7, 9);
    let culture = invariant();
    assert_eq!(value.to_string_format("d", &culture), "10/05/2024");
    assert_eq!(value.to_string_format("D", &culture), "Saturday, 05 October 2024");
    assert_eq!(value.to_string_format("t", &culture), "14:07");
    assert_eq!(value.to_string_format("T", &culture), "14:07:09");
    assert_eq!(value.to_string_format("f", &culture), "Saturday, 05 October 2024 14:07");
    assert_eq!(value.to_string_format("F", &culture), "Saturday, 05 October 2024 14:07:09");
    assert_eq!(value.to_string_format("g", &culture), "10/05/2024 14:07");
    assert_eq!(value.to_string_format("G", &culture), "10/05/2024 14:07:09");
    assert_eq!(value.to_string_format("", &culture), "10/05/2024 14:07:09");
    assert_eq!(value.to_string_format("M", &culture), "October 05");
    assert_eq!(value.to_string_format("Y", &culture), "2024 October");
    assert_eq!(value.to_string_format("y", &culture), "2024 October");
    assert_eq!(value.to_string_format("s", &culture), "2024-10-05T14:07:09");
    assert_eq!(value.to_string_format("u", &culture), "2024-10-05 14:07:09Z");
    assert_eq!(value.to_string_format("R", &culture), "Sat, 05 Oct 2024 14:07:09 GMT");
    assert_eq!(value.to_string_format("O", &culture), "2024-10-05T14:07:09.0000000");
    assert_eq!(value.specify_kind(DateTimeKind::Utc).to_string_format("o", &culture), "2024-10-05T14:07:09.0000000Z");
    assert_eq!(value.to_string_provider(&culture), "10/05/2024 14:07:09");
    assert!(value.try_to_string_format("Q", &culture).is_err());
}

#[test]
fn standard_formats_of_en_us() {
    let value = DateTime::new_with_time(2024, 10, 5, 14, 7, 9);
    let culture = en_us();
    assert_eq!(value.to_string_format("d", &culture), "10/5/2024");
    assert_eq!(value.to_string_format("D", &culture), "Saturday, October 5, 2024");
    assert_eq!(value.to_string_format("t", &culture), "2:07\u{202F}PM");
    assert_eq!(value.to_string_format("T", &culture), "2:07:09\u{202F}PM");
    assert_eq!(value.to_string_format("f", &culture), "Saturday, October 5, 2024 2:07\u{202F}PM");
    assert_eq!(value.to_string_format("F", &culture), "Saturday, October 5, 2024 2:07:09\u{202F}PM");
    assert_eq!(value.to_string_format("g", &culture), "10/5/2024 2:07\u{202F}PM");
    assert_eq!(value.to_string_format("G", &culture), "10/5/2024 2:07:09\u{202F}PM");
    assert_eq!(value.to_string_format("M", &culture), "October 5");
    assert_eq!(value.to_string_format("Y", &culture), "October 2024");
    // The culture-independent formats do not change.
    assert_eq!(value.to_string_format("s", &culture), "2024-10-05T14:07:09");
    assert_eq!(value.to_string_format("R", &culture), "Sat, 05 Oct 2024 14:07:09 GMT");
    assert_eq!(DateTime::new(2024, 1, 1).to_string_format("G", &culture), "1/1/2024 12:00:00\u{202F}AM");
}

#[test]
fn display_and_helpers_use_the_current_culture() {
    let value = DateTime::new_with_time(2024, 10, 5, 14, 7, 9);
    CultureInfo::set_current_culture(en_us());
    assert_eq!(value.to_string(), "10/5/2024 2:07:09\u{202F}PM");
    assert_eq!(value.to_string_with("yyyy-MM-dd"), "2024-10-05");
    CultureInfo::set_current_culture(invariant());
    assert_eq!(value.to_string(), "10/05/2024 14:07:09");
}

#[test]
fn custom_format_specifiers() {
    let value = DateTime::new_with_millisecond(2024, 3, 5, 15, 4, 9, 120);
    let culture = en_us();
    let format = |pattern: &str| value.to_string_format(pattern, &culture);
    // The patterns the pickers use.
    assert_eq!(format("%d"), "5");
    assert_eq!(format("dd"), "05");
    assert_eq!(format("ddd"), "Tue");
    assert_eq!(format("dddd"), "Tuesday");
    assert_eq!(format("%M"), "3");
    assert_eq!(format("MM"), "03");
    assert_eq!(format("MMM"), "Mar");
    assert_eq!(format("MMMM"), "March");
    assert_eq!(format("%y"), "24");
    assert_eq!(format("yy"), "24");
    assert_eq!(format("yyy"), "2024");
    assert_eq!(format("yyyy"), "2024");
    assert_eq!(format("yyyyy"), "02024");
    assert_eq!(format("%h"), "3");
    assert_eq!(format("hh"), "03");
    assert_eq!(format("%H"), "15");
    assert_eq!(format("HH"), "15");
    assert_eq!(format("%m"), "4");
    assert_eq!(format("mm"), "04");
    assert_eq!(format("%s"), "9");
    assert_eq!(format("ss"), "09");
    assert_eq!(format("%t"), "P");
    assert_eq!(format("tt"), "PM");
    assert_eq!(format("fff"), "120");
    assert_eq!(format("fffffff"), "1200000");
    assert_eq!(format("FFF"), "12");
    assert_eq!(format("gg"), "AD");
    // Combinations, literals and escapes.
    assert_eq!(format("yyyy-MM-dd"), "2024-03-05");
    assert_eq!(format("dd.MM.yyyy"), "05.03.2024");
    assert_eq!(format("MM dd yyyy"), "03 05 2024");
    assert_eq!(format("ddd, MMM d"), "Tue, Mar 5");
    assert_eq!(format("h:mm tt"), "3:04 PM");
    assert_eq!(format("yyyy/MM/dd HH:mm:ss.fff"), "2024/03/05 15:04:09.120");
    assert_eq!(format("'day' d 'of' MMMM"), "day 5 of March");
    assert_eq!(format("\"at\" HH\\h"), "at 15h");
    assert_eq!(format("d\\d"), "5d");
    assert_eq!(format("%d日"), "5日");
    assert_eq!(DateTime::new_with_time(2024, 3, 5, 0, 0, 0).to_string_format("%h tt", &culture), "12 AM");
    assert_eq!(DateTime::new_with_time(2024, 3, 5, 12, 0, 0).to_string_format("hh tt", &culture), "12 PM");
    assert_eq!(DateTime::new(2024, 3, 5).to_string_format("HH:mm:ss.FFF", &culture), "00:00:00");
    assert_eq!(DateTime::new(5, 3, 5).to_string_format("yyyy|yy|%y", &culture), "0005|05|5");
    // Invalid formats.
    for invalid in ["%", "%%", "'open", "trailing\\", "ffffffff"] {
        assert!(value.try_to_string_format(invalid, &culture).is_err(), "{invalid}");
    }
    assert!(panics(move || ignore(value.to_string_format("%", &CultureInfo::invariant_culture()))));
}

#[test]
fn separators_and_names_come_from_the_culture() {
    let value = DateTime::new_with_time(2024, 3, 5, 15, 4, 9);
    assert_eq!(value.to_string_format("yyyy/MM/dd HH:mm", &day_first()), "2024.03.05 15:04");
    assert_eq!(value.to_string_format("d", &day_first()), "05.03.2024");
    assert_eq!(value.to_string_format("D", &day_first()), "Tuesday, 5. March 2024");
    assert_eq!(value.to_string_format("d", &culture_of("en-GB")), "05/03/2024");
    assert_eq!(value.to_string_format("D", &year_first()), "2024年3月5日");
    assert_eq!(value.to_string_format("Y", &year_first()), "2024年3月");
    let mut info = DateTimeFormatInfo::new();
    info.set_month_names(std::array::from_fn(|index| format!("m{}", index + 1)));
    info.set_abbreviated_day_names(std::array::from_fn(|index| format!("d{index}")));
    assert_eq!(value.to_string_format("ddd MMMM", &info), "d2 m3");
}

#[test]
fn zone_specifiers() {
    pin_now(DateTime::new(2024, 1, 1), -5);
    let value = DateTime::new_with_time(2024, 3, 5, 15, 4, 9);
    let culture = invariant();
    assert_eq!(value.to_string_format("%z", &culture), "-5");
    assert_eq!(value.to_string_format("zz", &culture), "-05");
    assert_eq!(value.to_string_format("zzz", &culture), "-05:00");
    assert_eq!(value.to_string_format("HH:mmK", &culture), "15:04");
    assert_eq!(value.specify_kind(DateTimeKind::Utc).to_string_format("HH:mmK", &culture), "15:04Z");
    assert_eq!(value.specify_kind(DateTimeKind::Local).to_string_format("HH:mmK", &culture), "15:04-05:00");
    assert_eq!(value.specify_kind(DateTimeKind::Utc).to_string_format("zzz", &culture), "+00:00");
    assert_eq!(value.to_string_format("U", &culture), "Tuesday, 05 March 2024 20:04:09");
}

// --- DateTimeFormatInfo and CultureInfo ---

#[test]
fn format_info_of_the_invariant_culture() {
    let info = invariant().date_time_format();
    assert_eq!(info.short_date_pattern(), "MM/dd/yyyy");
    assert_eq!(info.long_date_pattern(), "dddd, dd MMMM yyyy");
    assert_eq!(info.short_time_pattern(), "HH:mm");
    assert_eq!(info.long_time_pattern(), "HH:mm:ss");
    assert_eq!(info.full_date_time_pattern(), "dddd, dd MMMM yyyy HH:mm:ss");
    assert_eq!(info.year_month_pattern(), "yyyy MMMM");
    assert_eq!(info.month_day_pattern(), "MMMM dd");
    assert_eq!(info.am_designator(), "AM");
    assert_eq!(info.pm_designator(), "PM");
    assert_eq!(info.first_day_of_week(), DayOfWeek::Sunday);
    assert_eq!(info.calendar_week_rule(), CalendarWeekRule::FirstDay);
    assert_eq!(info.shortest_day_names()[1], "Mo");
    assert_eq!(info.abbreviated_day_names()[6], "Sat");
    assert_eq!(info.day_names()[0], "Sunday");
    assert_eq!(info.month_names()[11], "December");
    assert_eq!(info.month_names()[12], "");
    assert_eq!(info.abbreviated_month_names()[0], "Jan");
    assert_eq!(info.get_month_name(2), "February");
    assert_eq!(info.get_abbreviated_month_name(9), "Sep");
    assert_eq!(info.get_day_name(DayOfWeek::Wednesday), "Wednesday");
    assert_eq!(info.get_abbreviated_day_name(DayOfWeek::Wednesday), "Wed");
    assert_eq!(info.get_shortest_day_name(DayOfWeek::Wednesday), "We");
    assert!(Rc::ptr_eq(&info, &DateTimeFormatInfo::invariant_info()));
}

#[test]
fn format_info_of_en_us_and_fallback() {
    // Without a provider every culture has the invariant conventions.
    assert!(Rc::ptr_eq(&CultureInfo::get_culture_info("en-US").date_time_format(), &DateTimeFormatInfo::invariant_info()));

    let info = en_us().date_time_format();
    assert_eq!(info.short_date_pattern(), "M/d/yyyy");
    assert_eq!(info.long_date_pattern(), "dddd, MMMM d, yyyy");
    assert_eq!(info.short_time_pattern(), "h:mm\u{202F}tt");
    assert_eq!(info.long_time_pattern(), "h:mm:ss\u{202F}tt");
    assert_eq!(info.year_month_pattern(), "MMMM yyyy");
    assert_eq!(info.first_day_of_week(), DayOfWeek::Sunday);

    // A culture without data uses its parent, then the invariant culture.
    assert!(Rc::ptr_eq(&CultureInfo::get_culture_info("en-AU").date_time_format(), &info));
    assert!(Rc::ptr_eq(&CultureInfo::get_culture_info("en").date_time_format(), &info));
    assert!(Rc::ptr_eq(&CultureInfo::get_culture_info("pl-PL").date_time_format(), &DateTimeFormatInfo::invariant_info()));

    let british = CultureInfo::get_culture_info("en-GB").date_time_format();
    assert_eq!(british.short_date_pattern(), "dd/MM/yyyy");
    assert_eq!(british.first_day_of_week(), DayOfWeek::Monday);
    assert_eq!(british.calendar_week_rule(), CalendarWeekRule::FirstFourDayWeek);

    CultureInfo::set_current_culture(en_us());
    assert!(Rc::ptr_eq(&DateTimeFormatInfo::current_info(), &info));
    CultureInfo::set_current_culture(invariant());
}

#[test]
fn a_culture_can_carry_modified_conventions() {
    let mut info = (*invariant().date_time_format()).clone();
    info.set_am_designator("");
    info.set_pm_designator("");
    info.set_short_date_pattern("yyyy-MM-dd");
    let culture = invariant().with_date_time_format(info);
    assert!(culture.is_invariant());
    assert_eq!(culture, invariant());
    assert_eq!(culture.date_time_format().am_designator(), "");
    assert_eq!(culture.date_time_format().pm_designator(), "");
    assert_eq!(invariant().date_time_format().am_designator(), "AM");
    let value = DateTime::new_with_time(2024, 3, 5, 15, 4, 9);
    assert_eq!(value.to_string_format("d", &culture), "2024-03-05");
    assert_eq!(value.to_string_format("h tt", &culture), "3 ");
    // The conventions themselves are a provider too.
    assert_eq!(value.to_string_format("d", &culture.date_time_format()), "2024-03-05");
    assert_eq!(value.to_string_format("d", &*culture.date_time_format()), "2024-03-05");
}

// --- parsing ---

#[test]
fn parse_exact_round_trips() {
    let culture = en_us();
    let value = DateTime::new_with_time(2024, 10, 17, 14, 7, 9);
    for format in ["yyyy-MM-dd HH:mm:ss", "G", "F", "s", "MM/dd/yyyy hh:mm:ss tt", "dddd, MMMM d, yyyy h:mm:ss tt", "o"] {
        let text = value.to_string_format(format, &culture);
        assert_eq!(DateTime::parse_exact(&text, format, &culture).unwrap(), value, "{format}: {text}");
    }
    let date = value.date();
    for format in ["d", "D", "yyyy-MM-dd", "MM dd yyyy", "dd.MM.yyyy", "d MMM yyyy", "yyyyMMdd", "ddd, dd MMM yyyy"] {
        let text = date.to_string_format(format, &culture);
        assert_eq!(DateTime::parse_exact(&text, format, &culture).unwrap(), date, "{format}: {text}");
    }
    let fraction = DateTime::new_with_millisecond(2024, 10, 17, 14, 7, 9, 120);
    assert_eq!(DateTime::parse_exact("14:07:09.12 17/10/2024", "HH:mm:ss.FFF dd/MM/yyyy", &culture).unwrap(), fraction);
    assert_eq!(DateTime::parse_exact("14:07:09.120 17/10/2024", "HH:mm:ss.fff dd/MM/yyyy", &culture).unwrap(), fraction);
}

#[test]
fn parse_exact_rejects_what_does_not_match() {
    let culture = en_us();
    assert_eq!(DateTime::parse_exact("17.10.2024", "dd.MM.yyyy", &culture).unwrap(), DateTime::new(2024, 10, 17));
    assert_eq!(DateTime::parse_exact("7.1.2024", "d.M.yyyy", &culture).unwrap(), DateTime::new(2024, 1, 7));
    for (text, format) in [
        ("2024-10-17", "dd.MM.yyyy"),
        ("17.10.2024 ", "dd.MM.yyyy"),
        (" 17.10.2024", "dd.MM.yyyy"),
        ("17.10.24", "dd.MM.yyyy"),
        ("7.10.2024", "dd.MM.yyyy"),
        ("31.02.2024", "dd.MM.yyyy"),
        ("17.13.2024", "dd.MM.yyyy"),
        ("17.10.2024x", "dd.MM.yyyy"),
        ("", "dd.MM.yyyy"),
        ("Monday, October 17, 2024", "D"),
        ("13:00 PM", "hh:mm tt"),
        ("25:00", "HH:mm"),
    ] {
        assert!(DateTime::parse_exact(text, format, &culture).is_err(), "{text} / {format}");
    }
    assert!(DateTime::parse_exact("17.10.2024", "", &culture).is_err());
    let styles = DateTimeStyles::ALLOW_WHITE_SPACES;
    assert_eq!(
        DateTime::parse_exact_with_styles("  17.10.2024 ", "dd.MM.yyyy", &culture, styles).unwrap(),
        DateTime::new(2024, 10, 17)
    );
    assert_eq!(DateTime::parse_exact("69", "yy", &culture).unwrap().year(), 1969);
    assert_eq!(DateTime::parse_exact("49", "yy", &culture).unwrap().year(), 2049);
    assert_eq!(DateTime::parse_exact("12 AM", "h tt", &culture).unwrap().hour(), 0);
    assert_eq!(DateTime::parse_exact("12 PM", "h tt", &culture).unwrap().hour(), 12);
    assert_eq!(DateTime::parse_exact("1 pm", "h tt", &culture).unwrap().hour(), 13);
}

#[test]
fn parse_exact_with_several_formats() {
    // The formats of the text converter of the upstream picker tests.
    let formats = ["yyyy-MM-dd", "MM dd yyyy", "dd.MM.yyyy"];
    let parse = |text: &str| DateTime::try_parse_exact_multiple(text, &formats, &invariant(), DateTimeStyles::NONE);
    assert_eq!(parse("17.10.2024"), Some(DateTime::new(2024, 10, 17)));
    assert_eq!(parse("2024-02-13"), Some(DateTime::new(2024, 2, 13)));
    assert_eq!(parse("04 22 2026"), Some(DateTime::new(2026, 4, 22)));
    assert_eq!(parse("22/04/2026"), None);
    assert_eq!(parse(""), None);
    assert_eq!(
        DateTime::try_parse_exact("2024-02-13", "yyyy-MM-dd", &invariant(), DateTimeStyles::NONE),
        Some(DateTime::new(2024, 2, 13))
    );
}

#[test]
fn parse_exact_defaults_missing_parts() {
    pin_now(DateTime::new_with_time(2024, 10, 12, 8, 0, 0), 0);
    let culture = invariant();
    assert_eq!(DateTime::parse_exact("14:30", "HH:mm", &culture).unwrap(), DateTime::new_with_time(2024, 10, 12, 14, 30, 0));
    assert_eq!(
        DateTime::parse_exact_with_styles("14:30", "HH:mm", &culture, DateTimeStyles::NO_CURRENT_DATE_DEFAULT).unwrap(),
        DateTime::new_with_time(1, 1, 1, 14, 30, 0)
    );
    assert_eq!(DateTime::parse_exact("March", "MMMM", &culture).unwrap(), DateTime::new(2024, 3, 1));
    assert_eq!(DateTime::parse_exact("03/05", "MM/dd", &culture).unwrap(), DateTime::new(2024, 3, 5));
    assert_eq!(DateTime::parse_exact("2020", "yyyy", &culture).unwrap(), DateTime::new(2020, 1, 1));
}

#[test]
fn parse_dates_in_the_order_of_the_culture() {
    pin_now(DateTime::new_with_time(2024, 10, 12, 8, 0, 0), 0);
    let expected = DateTime::new(2024, 10, 12);
    for text in [
        "10/12/2024",
        "10-12-2024",
        "10.12.2024",
        "10/12/24",
        " 10/12/2024 ",
        "2024-10-12",
        "2024/10/12",
        "October 12, 2024",
        "Oct 12, 2024",
        "Oct. 12, 2024",
        "12 October 2024",
        "12 Oct 2024",
        "Saturday, October 12, 2024",
        "Sat, 12 Oct 2024",
        "october 12 2024",
        "10/12",
        "Oct 12",
        "12 Oct",
        "2024-10-12T00:00:00",
    ] {
        assert_eq!(DateTime::parse(text, &en_us()).unwrap(), expected, "{text}");
    }
    assert_eq!(DateTime::parse("Oct 2024", &en_us()).unwrap(), DateTime::new(2024, 10, 1));
    assert_eq!(DateTime::parse("10/2024", &en_us()).unwrap(), DateTime::new(2024, 10, 1));
    assert_eq!(DateTime::parse("2024-10", &en_us()).unwrap(), DateTime::new(2024, 10, 1));

    // Day first.
    for culture in [culture_of("en-GB"), day_first()] {
        assert_eq!(DateTime::parse("12/10/2024", &culture).unwrap(), expected);
        assert_eq!(DateTime::parse("12.10.2024", &culture).unwrap(), expected);
        assert_eq!(DateTime::parse("2024-10-12", &culture).unwrap(), expected);
    }
    assert_eq!(DateTime::parse("12. October 2024", &day_first()).unwrap(), expected);
    // Year first.
    assert_eq!(DateTime::parse("2024/10/12", &year_first()).unwrap(), expected);
    assert_eq!(DateTime::parse("24/10/12", &year_first()).unwrap(), expected);
    assert_eq!(DateTime::parse("2024年10月12日", &year_first()).unwrap(), expected);
}

#[test]
fn parse_times() {
    pin_now(DateTime::new_with_time(2024, 10, 12, 8, 0, 0), 0);
    let culture = en_us();
    let at = |hour, minute, second| DateTime::new_with_time(2024, 10, 12, hour, minute, second);
    assert_eq!(DateTime::parse("10/12/2024 2:07:09 PM", &culture).unwrap(), at(14, 7, 9));
    assert_eq!(DateTime::parse("10/12/2024 14:07", &culture).unwrap(), at(14, 7, 0));
    assert_eq!(DateTime::parse("10/12/2024 12:00 AM", &culture).unwrap(), at(0, 0, 0));
    assert_eq!(DateTime::parse("10/12/2024 12:00 PM", &culture).unwrap(), at(12, 0, 0));
    assert_eq!(DateTime::parse("2:07 PM 10/12/2024", &culture).unwrap(), at(14, 7, 0));
    assert_eq!(DateTime::parse("October 12, 2024 5 pm", &culture).unwrap(), at(17, 0, 0));
    assert_eq!(DateTime::parse("2024-10-12T14:07:09", &culture).unwrap(), at(14, 7, 9));
    assert_eq!(
        DateTime::parse("2024-10-12T14:07:09.5", &culture).unwrap(),
        DateTime::new_with_millisecond(2024, 10, 12, 14, 7, 9, 500)
    );
    // A time alone gets the current date.
    assert_eq!(DateTime::parse("14:07", &culture).unwrap(), at(14, 7, 0));
    assert_eq!(DateTime::parse("2:07 PM", &culture).unwrap(), at(14, 7, 0));
    assert_eq!(
        DateTime::parse_with_styles("14:07", &culture, DateTimeStyles::NO_CURRENT_DATE_DEFAULT).unwrap(),
        DateTime::new_with_time(1, 1, 1, 14, 7, 0)
    );
    // The general format of each culture parses back.
    let value = at(14, 7, 9);
    for culture in [invariant(), en_us(), culture_of("en-GB"), day_first(), year_first()] {
        let name = culture.name().to_owned();
        for format in ["G", "g", "d", "D", "F", "f", "s", "u", "R", "Y"] {
            let text = value.to_string_format(format, &culture);
            let expected = match format {
                "d" | "D" => value.date(),
                "g" | "f" => at(14, 7, 0),
                "Y" => DateTime::new(2024, 10, 1),
                _ => value,
            };
            let parsed = DateTime::parse_with_styles(&text, &culture, DateTimeStyles::ADJUST_TO_UNIVERSAL);
            assert_eq!(parsed.unwrap(), expected, "{name} {format}: {text}");
        }
    }
}

#[test]
fn parse_rejects_malformed_text() {
    let culture = en_us();
    for text in [
        "", "   ", "abc", "12", "13/32/2024", "2/30/2024", "10/12/2024 25:00", "10/12/2024 13:00 AM",
        "10/12/2024 10:60", "Monday, October 12, 2024", "1/2/3/4", "Mayo 12, 2024", "10/12/2024 junk", ",",
    ] {
        assert!(DateTime::parse(text, &culture).is_err(), "{text:?}");
        assert!(DateTime::try_parse(text, &culture, DateTimeStyles::NONE).is_none(), "{text:?}");
    }
    assert!("garbage".parse::<DateTime>().is_err());
    CultureInfo::set_current_culture(culture_of("en-GB"));
    assert_eq!("10/12/2024".parse::<DateTime>().unwrap(), DateTime::new(2024, 12, 10));
    CultureInfo::set_current_culture(invariant());
}

#[test]
fn parse_offsets_and_kinds() {
    pin_now(DateTime::new(2024, 1, 1), 2);
    let culture = invariant();
    let plain = DateTime::parse("2024-10-12T14:00:00", &culture).unwrap();
    assert_eq!(plain.kind(), DateTimeKind::Unspecified);

    // A stated offset converts to local time...
    let local = DateTime::parse("2024-10-12T14:00:00Z", &culture).unwrap();
    assert_eq!((local, local.kind()), (DateTime::new_with_time(2024, 10, 12, 16, 0, 0), DateTimeKind::Local));
    let local = DateTime::parse("2024-10-12T14:00:00-05:00", &culture).unwrap();
    assert_eq!(local, DateTime::new_with_time(2024, 10, 12, 21, 0, 0));
    // ...unless the styles say otherwise.
    let utc = DateTime::parse_with_styles("2024-10-12T14:00:00+01:00", &culture, DateTimeStyles::ADJUST_TO_UNIVERSAL).unwrap();
    assert_eq!((utc, utc.kind()), (DateTime::new_with_time(2024, 10, 12, 13, 0, 0), DateTimeKind::Utc));
    let utc = DateTime::parse_with_styles("2024-10-12T14:00:00Z", &culture, DateTimeStyles::ROUNDTRIP_KIND).unwrap();
    assert_eq!((utc, utc.kind()), (DateTime::new_with_time(2024, 10, 12, 14, 0, 0), DateTimeKind::Utc));
    let assumed = DateTime::parse_with_styles("2024-10-12T14:00:00", &culture, DateTimeStyles::ASSUME_UNIVERSAL).unwrap();
    assert_eq!((assumed, assumed.kind()), (DateTime::new_with_time(2024, 10, 12, 16, 0, 0), DateTimeKind::Local));
    let assumed = DateTime::parse_with_styles("2024-10-12T14:00:00", &culture, DateTimeStyles::ASSUME_LOCAL).unwrap();
    assert_eq!((assumed, assumed.kind()), (DateTime::new_with_time(2024, 10, 12, 14, 0, 0), DateTimeKind::Local));

    let round_trip = DateTime::new_with_time(2024, 10, 12, 14, 0, 0).specify_kind(DateTimeKind::Utc);
    let text = round_trip.to_string_format("o", &culture);
    let parsed = DateTime::parse_exact_with_styles(&text, "o", &culture, DateTimeStyles::ROUNDTRIP_KIND).unwrap();
    assert_eq!((parsed, parsed.kind()), (round_trip, DateTimeKind::Utc));
}

// --- DateTimeOffset ---

#[test]
fn date_time_offset_construction_and_components() {
    let value = DateTimeOffset::new(2000, 10, 10, 1, 30, 0, TimeSpan::from_hours(2.0));
    assert_eq!(value.offset(), TimeSpan::from_hours(2.0));
    assert_eq!((value.year(), value.month(), value.day(), value.hour(), value.minute(), value.second()), (2000, 10, 10, 1, 30, 0));
    assert_eq!(value.date_time(), DateTime::new_with_time(2000, 10, 10, 1, 30, 0));
    assert_eq!(value.date_time().kind(), DateTimeKind::Unspecified);
    assert_eq!(value.utc_date_time(), DateTime::new_with_time(2000, 10, 9, 23, 30, 0));
    assert_eq!(value.utc_date_time().kind(), DateTimeKind::Utc);
    assert_eq!(value.date(), DateTime::new(2000, 10, 10));
    assert_eq!(value.time_of_day(), TimeSpan::from_hms(1, 30, 0));
    assert_eq!(value.day_of_week(), DayOfWeek::Tuesday);
    assert_eq!(value.day_of_year(), 284);
    assert_eq!(value.ticks() - value.utc_ticks(), TimeSpan::from_hours(2.0).ticks());

    assert_eq!(DateTimeOffset::MIN_VALUE.date_time(), DateTime::MIN_VALUE);
    assert_eq!(DateTimeOffset::MAX_VALUE.date_time(), DateTime::MAX_VALUE);
    assert_eq!(DateTimeOffset::MIN_VALUE.offset(), TimeSpan::ZERO);
    assert_eq!(DateTimeOffset::default(), DateTimeOffset::MIN_VALUE);

    assert!(DateTimeOffset::try_new(2000, 1, 1, 0, 0, 0, TimeSpan::from_hours(15.0)).is_none());
    assert!(DateTimeOffset::try_new(2000, 1, 1, 0, 0, 0, TimeSpan::from_seconds(30.0)).is_none());
    assert!(DateTimeOffset::try_new(1, 1, 1, 0, 0, 0, TimeSpan::from_hours(1.0)).is_none());
    assert!(DateTimeOffset::try_new(2000, 2, 30, 0, 0, 0, TimeSpan::ZERO).is_none());
    assert!(panics(|| ignore(DateTimeOffset::new(9999, 12, 31, 23, 0, 0, TimeSpan::from_hours(-2.0)))));
}

#[test]
fn date_time_offset_compares_instants() {
    let first = DateTimeOffset::new(2000, 10, 10, 12, 0, 0, TimeSpan::from_hours(2.0));
    let same = DateTimeOffset::new(2000, 10, 10, 10, 0, 0, TimeSpan::ZERO);
    let later = DateTimeOffset::new(2000, 10, 10, 11, 0, 0, TimeSpan::ZERO);
    assert_eq!(first, same);
    assert!(!first.equals_exact(same));
    assert!(first.equals_exact(same.to_offset(TimeSpan::from_hours(2.0))));
    assert!(first < later);
    assert_eq!(DateTimeOffset::compare(first, later), -1);
    assert_eq!(later.compare_to(first), 1);
    assert_eq!(later - first, TimeSpan::from_hours(1.0));
    // Optional values order the way the pickers compare them.
    assert!(Some(first) < Some(later));
}

#[test]
fn date_time_offset_arithmetic_keeps_the_offset() {
    let value = DateTimeOffset::new(2024, 1, 31, 12, 0, 0, TimeSpan::from_hours(-8.0));
    assert_eq!(value.add_months(1), DateTimeOffset::new(2024, 2, 29, 12, 0, 0, TimeSpan::from_hours(-8.0)));
    assert_eq!(value.add_months(1).offset(), TimeSpan::from_hours(-8.0));
    assert_eq!(value.add_years(1).date(), DateTime::new(2025, 1, 31));
    assert_eq!(value.add_days(1.0).day(), 1);
    assert_eq!((value + TimeSpan::from_minutes(30.0)).minute(), 30);
    assert_eq!((value - TimeSpan::from_hours(12.0)).hour(), 0);
    assert_eq!(value.to_universal_time().hour(), 20);
    assert_eq!(value.to_offset(TimeSpan::from_hours(1.0)).date_time(), DateTime::new_with_time(2024, 1, 31, 21, 0, 0));
    assert!(panics(|| ignore(DateTimeOffset::MAX_VALUE.add_days(1.0))));
    assert!(panics(|| ignore(DateTimeOffset::MAX_VALUE.to_offset(TimeSpan::from_hours(1.0)))));
}

#[test]
fn date_time_offset_from_date_time() {
    pin_now(DateTime::new(2024, 1, 1), 3);
    let clock = DateTime::new_with_time(2024, 6, 1, 12, 0, 0);
    assert_eq!(DateTimeOffset::from_date_time(clock).offset(), TimeSpan::from_hours(3.0));
    assert_eq!(DateTimeOffset::from(clock).date_time(), clock);
    assert_eq!(DateTimeOffset::from_date_time(clock.specify_kind(DateTimeKind::Utc)).offset(), TimeSpan::ZERO);
    assert_eq!(DateTimeOffset::from_date_time_offset(clock, TimeSpan::from_hours(-1.0)).utc_date_time().hour(), 13);
    assert!(panics(move || ignore(DateTimeOffset::from_date_time_offset(clock.specify_kind(DateTimeKind::Utc), TimeSpan::from_hours(1.0)))));
    let value = DateTimeOffset::new(2024, 6, 1, 12, 0, 0, TimeSpan::ZERO);
    assert_eq!(value.local_date_time(), DateTime::new_with_time(2024, 6, 1, 15, 0, 0));
    assert_eq!(value.local_date_time().kind(), DateTimeKind::Local);
    assert_eq!(value.to_local_time().offset(), TimeSpan::from_hours(3.0));
    assert_eq!(DateTimeOffset::from_ticks(clock.ticks(), TimeSpan::ZERO).date_time(), clock);
}

#[test]
fn date_time_offset_formatting() {
    let value = DateTimeOffset::new(2000, 10, 10, 9, 5, 0, TimeSpan::from_hms(-5, -30, 0));
    assert_eq!(value.to_string_format("", &invariant()), "10/10/2000 09:05:00 -05:30");
    assert_eq!(value.to_string_format("", &en_us()), "10/10/2000 9:05:00\u{202F}AM -05:30");
    assert_eq!(value.to_string_provider(&en_us()), "10/10/2000 9:05:00\u{202F}AM -05:30");
    assert_eq!(value.to_string_format("G", &en_us()), "10/10/2000 9:05:00\u{202F}AM");
    // The patterns of the date picker.
    assert_eq!(value.to_string_format("MMMM", &en_us()), "October");
    assert_eq!(value.to_string_format("yyyy", &en_us()), "2000");
    assert_eq!(value.to_string_format("%d", &en_us()), "10");
    assert_eq!(value.to_string_format("ddd", &en_us()), "Tue");
    assert_eq!(value.to_string_format("{0}", &en_us()), "{0}");
    assert_eq!(value.to_string_format("zzz|zz|%z|%K", &en_us()), "-05:30|-05|-5|-05:30");
    assert_eq!(value.to_string_format("o", &en_us()), "2000-10-10T09:05:00.0000000-05:30");
    assert_eq!(value.to_string_format("u", &en_us()), "2000-10-10 14:35:00Z");
    assert_eq!(value.to_string_format("R", &en_us()), "Tue, 10 Oct 2000 14:35:00 GMT");
    assert!(value.try_to_string_format("U", &en_us()).is_err());
    let utc = DateTimeOffset::new(2000, 10, 10, 0, 0, 0, TimeSpan::ZERO);
    assert_eq!(utc.to_string_format("", &en_us()), "10/10/2000 12:00:00\u{202F}AM +00:00");
    CultureInfo::set_current_culture(en_us());
    assert_eq!(utc.to_string(), "10/10/2000 12:00:00\u{202F}AM +00:00");
    assert_eq!(utc.to_string_with("yyyy"), "2000");
    CultureInfo::set_current_culture(invariant());
}

// --- calendar ---

#[test]
fn gregorian_calendar_arithmetic() {
    let calendar = GregorianCalendar::new();
    assert_eq!(calendar.get_days_in_month(2024, 2), 29);
    assert_eq!(calendar.get_day_of_week(DateTime::new(2021, 1, 4)), DayOfWeek::Monday);
    assert_eq!(calendar.add_days(DateTime::new(2024, 2, 28), 2), DateTime::new(2024, 3, 1));
    assert_eq!(calendar.add_months(DateTime::new(2024, 1, 31), 1), DateTime::new(2024, 2, 29));
    assert_eq!(calendar.add_years(DateTime::new(2024, 2, 29), -1), DateTime::new(2023, 2, 28));
    assert_eq!(calendar.try_add_days(DateTime::MAX_VALUE, 1), None);
    assert_eq!(calendar.try_add_months(DateTime::MIN_VALUE, -1), None);
    assert_eq!(calendar.try_add_years(DateTime::new(9999, 1, 1), 1), None);
    assert_eq!(calendar.try_add_years(DateTime::new(2000, 1, 1), i32::MAX), None);
    assert_eq!(calendar.min_supported_date_time(), DateTime::MIN_VALUE);
    assert_eq!(calendar.max_supported_date_time(), DateTime::MAX_VALUE);
    assert_eq!(CultureInfo::invariant_culture().calendar(), calendar);
}

#[test]
fn week_of_year_by_rule() {
    let calendar = GregorianCalendar::new();
    let week = |year, month, day, rule, first| calendar.get_week_of_year(DateTime::new(year, month, day), rule, first);
    use CalendarWeekRule::{FirstDay, FirstFourDayWeek, FirstFullWeek};
    // The rows of the upstream calendar tests that reach the calendar.
    assert_eq!(week(2023, 1, 1, FirstDay, DayOfWeek::Sunday), 1);
    assert_eq!(week(2023, 12, 31, FirstDay, DayOfWeek::Sunday), 53);
    assert_eq!(week(2023, 1, 7, FirstDay, DayOfWeek::Sunday), 1);
    assert_eq!(week(2023, 1, 8, FirstDay, DayOfWeek::Sunday), 2);
    assert_eq!(week(2021, 1, 1, FirstDay, DayOfWeek::Monday), 1);
    assert_eq!(week(2021, 1, 4, FirstDay, DayOfWeek::Monday), 2);
    assert_eq!(week(2021, 1, 1, FirstFullWeek, DayOfWeek::Monday), 52);
    assert_eq!(week(2021, 1, 4, FirstFullWeek, DayOfWeek::Monday), 1);
    assert_eq!(week(2021, 1, 1, FirstFourDayWeek, DayOfWeek::Monday), 53);
    assert_eq!(week(2021, 1, 4, FirstFourDayWeek, DayOfWeek::Monday), 1);
    assert_eq!(week(2023, 1, 2, FirstFourDayWeek, DayOfWeek::Monday), 1);
    assert_eq!(week(2022, 12, 31, FirstFourDayWeek, DayOfWeek::Monday), 52);
    assert_eq!(week(2020, 1, 1, FirstFourDayWeek, DayOfWeek::Monday), 1);
    // The documented difference from ISO 8601: the end of December is never week 1.
    assert_eq!(week(2018, 12, 31, FirstFourDayWeek, DayOfWeek::Monday), 53);
    assert_eq!(week(2019, 12, 30, FirstFourDayWeek, DayOfWeek::Monday), 53);
    assert_eq!(week(2023, 1, 1, FirstFourDayWeek, DayOfWeek::Sunday), 1);
    assert_eq!(week(2022, 1, 1, FirstFourDayWeek, DayOfWeek::Sunday), 52);
    // The first days of year one.
    assert_eq!(week(1, 1, 1, FirstDay, DayOfWeek::Sunday), 1);
    assert_eq!(week(1, 1, 1, FirstFourDayWeek, DayOfWeek::Monday), 1);
    assert_eq!(week(1, 1, 1, FirstFullWeek, DayOfWeek::Sunday), 53);
    assert_eq!(week(1, 1, 7, FirstFullWeek, DayOfWeek::Sunday), 1);
}

#[test]
fn iso_week_numbers() {
    let week = |year, month, day| IsoWeek::get_week_of_year(DateTime::new(year, month, day));
    assert_eq!(week(2023, 1, 2), 1);
    assert_eq!(week(2022, 12, 31), 52);
    assert_eq!(week(2018, 12, 31), 1);
    assert_eq!(week(2020, 1, 1), 1);
    assert_eq!(week(2014, 12, 29), 1);
    assert_eq!(week(2019, 12, 30), 1);
    assert_eq!(week(2021, 1, 1), 53);
    assert_eq!(week(2021, 1, 4), 1);
    assert_eq!(week(2020, 12, 31), 53);
    assert_eq!(week(2023, 1, 1), 52);
}

// --- small types ---

#[test]
fn day_of_week_values() {
    assert_eq!(DayOfWeek::Sunday as i32, 0);
    assert_eq!(DayOfWeek::Saturday.to_i32(), 6);
    assert_eq!(DayOfWeek::from_i32(1), Some(DayOfWeek::Monday));
    assert_eq!(DayOfWeek::from_i32(7), None);
    assert_eq!(DayOfWeek::from_i32(-1), None);
    assert_eq!(DayOfWeek::Wednesday.to_string(), "Wednesday");
    assert_eq!(DayOfWeek::default(), DayOfWeek::Sunday);
    assert_eq!(i32::from(DayOfWeek::Thursday), 4);
}

#[test]
fn time_span_members_of_the_pickers() {
    let value = TimeSpan::from_hms(13, 5, 9);
    assert_eq!((value.days(), value.hours(), value.minutes(), value.seconds(), value.milliseconds()), (0, 13, 5, 9, 0));
    assert_eq!(TimeSpan::from_hours(10.0), TimeSpan::from_hms(10, 0, 0));
    assert_eq!(TimeSpan::from_days(1.5), TimeSpan::from_dhms(1, 12, 0, 0));
    assert_eq!(TimeSpan::from_dhms_milliseconds(1, 2, 3, 4, 5).ticks(), 937_840_050_000);
    assert_eq!(TimeSpan::from_dhms(1, 12, 0, 0).total_days(), 1.5);
    assert_eq!(TimeSpan::from_hms(1, 30, 0).total_hours(), 1.5);
    assert_eq!(TimeSpan::from_hms(0, 1, 30).total_minutes(), 1.5);
    assert_eq!(TimeSpan::from_hms(-1, -30, 0).duration(), TimeSpan::from_hms(1, 30, 0));
    assert_eq!(TimeSpan::from_hms(-1, -30, 0).hours(), -1);
    assert_eq!(TimeSpan::from_hms(-1, -30, 0).minutes(), -30);
    assert_eq!(TimeSpan::from_hms(25, 0, 0).days(), 1);

    // The formats of the time picker.
    assert_eq!(value.to_string_format("%h"), "13");
    assert_eq!(TimeSpan::from_hms(3, 5, 9).to_string_format("%h"), "3");
    assert_eq!(TimeSpan::from_hms(3, 5, 9).to_string_format("hh"), "03");
    assert_eq!(value.to_string_format("mm"), "05");
    assert_eq!(value.to_string_format("%m"), "5");
    assert_eq!(value.to_string_format("ss"), "09");
    assert_eq!(value.to_string_format("%s"), "9");
    assert_eq!(TimeSpan::from_hms(0, 0, 0).to_string_format("mm"), "00");
    assert_eq!(value.to_string_format("hh\\:mm\\:ss"), "13:05:09");
    assert_eq!(value.to_string_format("h'h 'm'm'"), "13h 5m");
    assert_eq!(TimeSpan::from_dhms_milliseconds(2, 3, 4, 5, 60).to_string_format("d\\.hh\\:mm\\:ss\\.fff"), "2.03:04:05.060");
    assert_eq!(TimeSpan::from_dhms_milliseconds(0, 0, 0, 5, 60).to_string_format("s\\.FFF"), "5.06");
    assert_eq!(TimeSpan::from_hms(-3, 0, 0).to_string_format("hh"), "03");
    // Standard formats.
    assert_eq!(value.to_string_format(""), "13:05:09");
    assert_eq!(value.to_string_format("c"), "13:05:09");
    assert_eq!(value.to_string_format("g"), "13:05:09");
    assert_eq!(TimeSpan::from_dhms_milliseconds(1, 3, 5, 9, 500).to_string_format("g"), "1:3:05:09.5");
    assert_eq!(TimeSpan::from_hms(3, 5, 9).to_string_format("G"), "0:03:05:09.0000000");
    assert_eq!(TimeSpan::from_hms(-3, -5, -9).to_string_format("g"), "-3:05:09");
    // Invalid formats.
    for invalid in ["x", "hh:mm", "hhh", "mmm", "sss", "%", "%%", "'open", "end\\", "HH"] {
        assert!(value.try_to_string_format(invalid).is_err(), "{invalid}");
    }
    assert!(panics(move || ignore(value.to_string_format("hh:mm"))));
}

/// The results of .NET 10 `DateTimeOffset.Parse(text, CultureInfo.InvariantCulture)`
/// with the local time zone at +02:00: the ticks of the clock time and the
/// offset in minutes, or `None` where .NET throws `FormatException`.
const DATE_TIME_OFFSET_PARSE: &[(&str, Option<(i64, i64)>)] = &[
    ("2000-10-10T12:30:45+02:00", Some((631067778450000000, 120))),
    ("2000-10-10T12:30:45Z", Some((631067778450000000, 0))),
    ("2000-10-10T12:30:45", Some((631067778450000000, 120))),
    ("2000-10-10", Some((631067328000000000, 120))),
    ("10/10/2000", Some((631067328000000000, 120))),
    ("10/10/2000 14:05", Some((631067835000000000, 120))),
    ("10/10/2000 2:05 PM -05:00", Some((631067835000000000, -300))),
    ("2000-10-10T12:30:45.1234567-08:00", Some((631067778451234567, -480))),
    ("2000-10-10 12:30:45 +05:30", Some((631067778450000000, 330))),
    ("Tue, 10 Oct 2000 12:30:45 GMT", Some((631067778450000000, 0))),
    ("2000-10-10T12:30:45+15:00", None),
    ("", None),
    ("garbage", None),
    ("0001-01-01T00:00:00+01:00", None),
    ("2000-10-10T12:30:45 -0800", Some((631067778450000000, -480))),
    ("2000-10-10T12:30+02", Some((631067778000000000, 120))),
    ("  2000-10-10T12:30:45+02:00  ", Some((631067778450000000, 120))),
    ("10 October 2000", Some((631067328000000000, 120))),
    ("2000-10-10T12:30:45-00:00", Some((631067778450000000, 0))),
    ("2000-10-10T12:30:45+14:00", Some((631067778450000000, 840))),
    ("2000-10-10T12:30:45-14:00", Some((631067778450000000, -840))),
    ("2000-10-10T12:30:45+14:01", None),
    ("9999-12-31T23:59:59-01:00", None),
    ("2000-10-10T12:30:45+02:30", Some((631067778450000000, 150))),
    ("2000-10-10 12:30:45Z", Some((631067778450000000, 0))),
    ("2000-02-30", None),
    ("10/10/2000 12:30:45 +2", Some((631067778450000000, 120))),
    ("2000-10-10T12:30:45.5+01:00", Some((631067778455000000, 60))),
    ("2000-10-10T12:30:45+02:00:30", None),
];

/// More results of .NET 10 with the local time zone at +02:00 and the
/// current date 2026-10-05: of `DateTimeOffset.Parse` as above, and of
/// `DateTime.Parse(text, CultureInfo.InvariantCulture)` (the ticks and the
/// kind). Offsets and the UTC names around dates without a time.
const OFFSET_PARSE: &[(&str, Option<(i64, i64)>, Option<(i64, u8)>)] = &[
    ("10/10/2000 +02:00", Some((631067328000000000, 120)), Some((631067328000000000, 2))),
    ("October 10, 2000 +01:00", Some((631067328000000000, 60)), Some((631067364000000000, 2))),
    ("2000-10-10 +0200", Some((631067328000000000, 120)), Some((631067328000000000, 2))),
    ("2000-10-10 -08:00", Some((631067328000000000, -480)), Some((631067688000000000, 2))),
    ("10/10/2000 -0800", Some((631067328000000000, -480)), Some((631067688000000000, 2))),
    ("2000-10-10 +02", Some((631067328000000000, 120)), Some((631067328000000000, 2))),
    ("10/10/2000 +2", Some((631067328000000000, 120)), Some((631067328000000000, 2))),
    ("10/10/2000 -2", Some((631067328000000000, -120)), Some((631067472000000000, 2))),
    ("2000-10-10+02:00", Some((631067328000000000, 120)), Some((631067328000000000, 2))),
    ("2000-10-10-08:00", Some((631067328000000000, -480)), Some((631067688000000000, 2))),
    ("10/10/2000+02:00", Some((631067328000000000, 120)), Some((631067328000000000, 2))),
    ("+02:00 10/10/2000", Some((631067328000000000, 120)), Some((631067328000000000, 2))),
    ("10 Oct 2000 -05:00", Some((631067328000000000, -300)), Some((631067580000000000, 2))),
    ("Oct 2000 +01:00", Some((631059552000000000, 60)), Some((631059588000000000, 2))),
    ("2000-10-10 Z", Some((631067328000000000, 0)), Some((631067400000000000, 2))),
    ("2000-10-10Z", Some((631067328000000000, 0)), Some((631067400000000000, 2))),
    ("10/10/2000 GMT", Some((631067328000000000, 0)), Some((631067400000000000, 2))),
    ("10/10/2000 UTC", None, None),
    ("2000-10-10T12:30:45 UTC", None, None),
    ("2000-10-10T12:30:45 GMT", None, None),
    ("2000-10-10 12:30:45 UTC", None, None),
    ("Tue, 10 Oct 2000 12:30:45 UTC", None, None),
    ("12:30 UTC", None, None),
    ("12:30 GMT", Some((639268002000000000, 0)), Some((639268074000000000, 2))),
    ("2000-10-10T12:30:45UTC", None, None),
    ("2000-10-10T12:30:45GMT", None, None),
    ("10/10/2000 12:30 utc", None, None),
    ("10/10/2000 12:30 gmt", Some((631067778000000000, 0)), Some((631067850000000000, 2))),
    ("10/10/2000 12:30 PM UTC", None, None),
    ("10/10/2000 +02:00 12:30", Some((631067778000000000, 120)), Some((631067778000000000, 2))),
    ("10/10/2000 -02:00 12:30", Some((631067778000000000, -120)), Some((631067922000000000, 2))),
    ("2000-10-10 +15:00", None, None),
    ("10/10/2000 + 02:00", None, None),
    ("10/10/2000 +02:00 +01:00", None, None),
    ("12:30 +02:00", Some((639268002000000000, 120)), Some((639268002000000000, 2))),
    ("12:30 -02:00", Some((639268002000000000, -120)), Some((639268146000000000, 2))),
    ("10/10/2000 12:30 -2", Some((631067778000000000, -120)), Some((631067922000000000, 2))),
    ("10/10/2000 -2:30", Some((631067328000000000, -150)), Some((631067490000000000, 2))),
    ("10/10/2000 +2:30", Some((631067328000000000, 150)), Some((631067310000000000, 2))),
    ("10/10/2000 +0230", Some((631067328000000000, 150)), Some((631067310000000000, 2))),
    ("10/10/2000 +230", Some((631067328000000000, 150)), Some((631067310000000000, 2))),
];

#[test]
fn date_time_offset_parse_matches_net() {
    pin_now(DateTime::new_with_time(2026, 10, 5, 10, 0, 0), 2);

    let mut failures = Vec::new();
    for (text, expected) in DATE_TIME_OFFSET_PARSE {
        let actual = DateTimeOffset::parse(text, &CultureInfo::invariant_culture()).ok().map(|value| (value.ticks(), value.offset().ticks() / TimeSpan::TICKS_PER_MINUTE));
        if actual != *expected {
            failures.push(format!("{text:?}: expected {expected:?}, got {actual:?}"));
        }
        assert_eq!(text.parse::<DateTimeOffset>().ok().map(|value| value.ticks()), actual.map(|(ticks, _)| ticks));
    }

    for (text, expected, expected_date_time) in OFFSET_PARSE {
        let actual = DateTimeOffset::parse(text, &CultureInfo::invariant_culture()).ok().map(|value| (value.ticks(), value.offset().ticks() / TimeSpan::TICKS_PER_MINUTE));
        if actual != *expected {
            failures.push(format!("{text:?}: expected {expected:?}, got {actual:?}"));
        }
        let actual = DateTime::parse(text, &CultureInfo::invariant_culture()).ok().map(|value| (value.ticks(), value.kind() as u8));
        if actual != *expected_date_time {
            failures.push(format!("DateTime {text:?}: expected {expected_date_time:?}, got {actual:?}"));
        }
    }

    DateTime::set_utc_now_provider(None);
    TimeZoneInfo::set_local_utc_offset_provider(None);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn date_time_offset_parses_the_stated_offset_or_the_local_one() {
    let invariant = CultureInfo::invariant_culture();
    // An offset in the text is the offset of the value.
    let parsed = DateTimeOffset::parse("2024-03-05T14:30:15+02:00", &invariant).unwrap();
    assert!(parsed.equals_exact(DateTimeOffset::new(2024, 3, 5, 14, 30, 15, TimeSpan::from_hours(2.0))));
    let parsed = DateTimeOffset::parse("2024-03-05T14:30:15.5-05:30", &invariant).unwrap();
    assert_eq!(parsed.offset(), TimeSpan::from_minutes(-330.0));
    assert_eq!((parsed.hour(), parsed.minute(), parsed.second(), parsed.millisecond()), (14, 30, 15, 500));
    // The UTC designator is the offset zero.
    let parsed = DateTimeOffset::parse("2024-03-05T14:30:15Z", &invariant).unwrap();
    assert!(parsed.equals_exact(DateTimeOffset::new(2024, 3, 5, 14, 30, 15, TimeSpan::ZERO)));
    // Without an offset the text is a local time.
    let parsed = DateTimeOffset::parse("03/05/2024 14:30:15", &invariant).unwrap();
    let clock = DateTime::new_with_time(2024, 3, 5, 14, 30, 15);
    assert!(parsed.equals_exact(DateTimeOffset::from_date_time(clock)));
    assert_eq!(parsed.date_time(), clock);
    // A date alone is midnight.
    let parsed = DateTimeOffset::parse("2024-03-05", &invariant).unwrap();
    assert_eq!(parsed.date_time(), DateTime::new_with_time(2024, 3, 5, 0, 0, 0));

    // What is not a date, an offset beyond 14 hours and an instant out of range are format errors.
    assert!(DateTimeOffset::parse("not a date", &invariant).is_err());
    assert!(DateTimeOffset::parse("", &invariant).is_err());
    assert!(DateTimeOffset::parse("2024-03-05T14:30:15+15:00", &invariant).is_err());
    assert!(DateTimeOffset::parse("0001-01-01T00:00:00+01:00", &invariant).is_err());
    assert_eq!(DateTimeOffset::try_parse("not a date", &invariant), None);
    assert!(DateTimeOffset::try_parse("2024-03-05T14:30:15+02:00", &invariant).is_some());
}

