//! Checks the date and time types against results recorded from .NET
//! (`date_time_net_data.rs`): culture data, formatting, parsing, week
//! numbers and arithmetic.

use super::date_time_net_data as net;
use super::{
    CalendarWeekRule, CultureInfo, DateTime, DateTimeKind, DateTimeOffset, DateTimeStyles, DayOfWeek,
    GregorianCalendar, IsoWeek, TestCultureDataProvider,
};
use crate::animation::TimeSpan;
use std::rc::Rc;

/// The cultures of the tables, with the test culture data registered, the
/// clock pinned to the day of the recording and the local offset zero.
fn cultures() -> [CultureInfo; 4] {
    TestCultureDataProvider::register();
    let (year, month, day) = net::NOW;
    let now = DateTime::new_with_time(year, month, day, 12, 0, 0).specify_kind(DateTimeKind::Utc);
    DateTime::set_utc_now_provider(Some(Rc::new(move || now)));
    net::CULTURES.map(CultureInfo::get_culture_info)
}

fn kind(value: u8) -> DateTimeKind {
    match value {
        1 => DateTimeKind::Utc,
        2 => DateTimeKind::Local,
        _ => DateTimeKind::Unspecified,
    }
}

fn parsed(result: Option<DateTime>) -> Option<(i64, u8)> {
    result.map(|value| (value.ticks(), value.kind() as u8))
}

/// Fails with the mismatches of a table.
fn report(table: &str, rows: usize, mismatches: Vec<String>) {
    assert!(
        mismatches.is_empty(),
        "{table}: {} of {rows} rows differ from .NET:\n{}",
        mismatches.len(),
        mismatches.iter().take(60).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn date_time_format_info_matches() {
    let cultures = cultures();
    let mut mismatches = Vec::new();
    for &(culture, member, expected) in net::DATE_TIME_FORMAT_INFO {
        let info = cultures[culture as usize].date_time_format();
        let actual = match member {
            "ShortDatePattern" => info.short_date_pattern().to_owned(),
            "LongDatePattern" => info.long_date_pattern().to_owned(),
            "ShortTimePattern" => info.short_time_pattern().to_owned(),
            "LongTimePattern" => info.long_time_pattern().to_owned(),
            "FullDateTimePattern" => info.full_date_time_pattern().into_owned(),
            "MonthDayPattern" => info.month_day_pattern().to_owned(),
            "YearMonthPattern" => info.year_month_pattern().to_owned(),
            "AMDesignator" => info.am_designator().to_owned(),
            "PMDesignator" => info.pm_designator().to_owned(),
            "DateSeparator" => info.date_separator().to_owned(),
            "TimeSeparator" => info.time_separator().to_owned(),
            "FirstDayOfWeek" => (info.first_day_of_week() as i32).to_string(),
            "CalendarWeekRule" => (info.calendar_week_rule() as i32).to_string(),
            "Calendar" => "GregorianCalendar".to_owned(),
            "TwoDigitYearMax" => "2049".to_owned(),
            "DayNames" => info.day_names().join("|"),
            "AbbreviatedDayNames" => info.abbreviated_day_names().join("|"),
            "ShortestDayNames" => info.shortest_day_names().join("|"),
            // The genitive names equal the plain ones in these cultures; they are not modelled.
            "MonthNames" | "MonthGenitiveNames" => info.month_names().join("|"),
            "AbbreviatedMonthNames" | "AbbreviatedMonthGenitiveNames" => info.abbreviated_month_names().join("|"),
            "RFC1123Pattern" => info.rfc1123_pattern().to_owned(),
            "SortableDateTimePattern" => info.sortable_date_time_pattern().to_owned(),
            "UniversalSortableDateTimePattern" => info.universal_sortable_date_time_pattern().to_owned(),
            "EraName" => info.get_era_name(1).to_owned(),
            other => panic!("unknown member {other}"),
        };
        if actual != expected {
            mismatches.push(format!("{:?} {member}: {actual:?}, .NET {expected:?}", net::CULTURES[culture as usize]));
        }
    }
    report("DATE_TIME_FORMAT_INFO", net::DATE_TIME_FORMAT_INFO.len(), mismatches);
}

#[test]
fn number_format_info_matches() {
    let cultures = cultures();
    let mut mismatches = Vec::new();
    let sizes = |sizes: &[i32]| sizes.iter().map(i32::to_string).collect::<Vec<_>>().join(",");
    for &(culture, member, expected) in net::NUMBER_FORMAT_INFO {
        let info = cultures[culture as usize].number_format();
        let actual = match member {
            "CurrencyDecimalDigits" => info.currency_decimal_digits().to_string(),
            "CurrencyDecimalSeparator" => info.currency_decimal_separator().to_owned(),
            "CurrencyGroupSeparator" => info.currency_group_separator().to_owned(),
            "CurrencyGroupSizes" => sizes(info.currency_group_sizes()),
            "CurrencyNegativePattern" => info.currency_negative_pattern().to_string(),
            "CurrencyPositivePattern" => info.currency_positive_pattern().to_string(),
            "CurrencySymbol" => info.currency_symbol().to_owned(),
            "NaNSymbol" => info.nan_symbol().to_owned(),
            "NegativeInfinitySymbol" => info.negative_infinity_symbol().to_owned(),
            "NegativeSign" => info.negative_sign().to_owned(),
            "NumberDecimalDigits" => info.number_decimal_digits().to_string(),
            "NumberDecimalSeparator" => info.number_decimal_separator().to_owned(),
            "NumberGroupSeparator" => info.number_group_separator().to_owned(),
            "NumberGroupSizes" => sizes(info.number_group_sizes()),
            "NumberNegativePattern" => info.number_negative_pattern().to_string(),
            "PercentDecimalDigits" => info.percent_decimal_digits().to_string(),
            "PercentDecimalSeparator" => info.percent_decimal_separator().to_owned(),
            "PercentGroupSeparator" => info.percent_group_separator().to_owned(),
            "PercentGroupSizes" => sizes(info.percent_group_sizes()),
            "PercentNegativePattern" => info.percent_negative_pattern().to_string(),
            "PercentPositivePattern" => info.percent_positive_pattern().to_string(),
            "PercentSymbol" => info.percent_symbol().to_owned(),
            "PerMilleSymbol" => info.per_mille_symbol().to_owned(),
            "PositiveInfinitySymbol" => info.positive_infinity_symbol().to_owned(),
            "PositiveSign" => info.positive_sign().to_owned(),
            other => panic!("unknown member {other}"),
        };
        if actual != expected {
            mismatches.push(format!("{:?} {member}: {actual:?}, .NET {expected:?}", net::CULTURES[culture as usize]));
        }
    }
    report("NUMBER_FORMAT_INFO", net::NUMBER_FORMAT_INFO.len(), mismatches);
}

#[test]
fn formatting_matches() {
    let cultures = cultures();
    let mut mismatches = Vec::new();
    for &(culture, ticks, format, expected) in net::FORMAT {
        // `en` gives the same as `en-US`: check both against the row.
        let targets: &[usize] = if culture == 2 { &[1, 2] } else { &[culture as usize] };
        for &target in targets {
            let actual = DateTime::from_ticks(ticks).try_to_string_format(format, &cultures[target]).ok();
            if actual.as_deref() != expected {
                mismatches.push(format!("{:?} {ticks} {format:?}: {actual:?}, .NET {expected:?}", net::CULTURES[target]));
            }
        }
    }
    report("FORMAT", net::FORMAT.len(), mismatches);
}

#[test]
fn formatting_by_kind_matches() {
    let cultures = cultures();
    let mut mismatches = Vec::new();
    for &(value_kind, ticks, format, expected) in net::FORMAT_KIND {
        let actual = DateTime::from_ticks_kind(ticks, kind(value_kind)).to_string_format(format, &cultures[0]);
        if actual != expected {
            mismatches.push(format!("kind {value_kind} {format:?}: {actual:?}, .NET {expected:?}"));
        }
    }
    report("FORMAT_KIND", net::FORMAT_KIND.len(), mismatches);
}

#[test]
fn date_time_offset_formatting_matches() {
    let cultures = cultures();
    let mut mismatches = Vec::new();
    for &(culture, ticks, offset_minutes, format, expected) in net::FORMAT_OFFSET {
        let offset = TimeSpan::from_ticks(offset_minutes as i64 * TimeSpan::TICKS_PER_MINUTE);
        let value = DateTimeOffset::from_ticks(ticks, offset);
        let actual = value.try_to_string_format(format, &cultures[culture as usize]).ok();
        if actual.as_deref() != expected {
            mismatches.push(format!(
                "{:?} {ticks} {offset_minutes} {format:?}: {actual:?}, .NET {expected:?}",
                net::CULTURES[culture as usize]
            ));
        }
    }
    report("FORMAT_OFFSET", net::FORMAT_OFFSET.len(), mismatches);
}

#[test]
fn week_numbers_match() {
    let calendar = GregorianCalendar::new();
    let rules = [CalendarWeekRule::FirstDay, CalendarWeekRule::FirstFullWeek, CalendarWeekRule::FirstFourDayWeek];
    let mut mismatches = Vec::new();
    for &(year, month, day, day_of_week, day_of_year, iso_week, weeks) in net::WEEK {
        let date = DateTime::new(year, month, day);
        if (date.day_of_week() as i32, date.day_of_year(), IsoWeek::get_week_of_year(date)) != (day_of_week, day_of_year, iso_week) {
            mismatches.push(format!("{year}-{month}-{day}: day of week, day of year or ISO week differ"));
        }
        for (index, expected) in weeks.iter().enumerate() {
            let (rule, first) = (rules[index / 7], DayOfWeek::ALL[index % 7]);
            let actual = calendar.get_week_of_year(date, rule, first);
            if actual != *expected as i32 {
                mismatches.push(format!("{year}-{month}-{day} {rule:?} {first:?}: {actual}, .NET {expected}"));
            }
        }
    }
    report("WEEK", net::WEEK.len(), mismatches);
}

#[test]
fn arithmetic_matches() {
    let calendar = GregorianCalendar::new();
    let ticks = |value: Option<DateTime>| value.map(DateTime::ticks);
    let mut mismatches = Vec::new();
    for &(start, months, expected, calendar_expected) in net::ADD_MONTHS {
        let value = DateTime::from_ticks(start);
        let actual = (ticks(value.try_add_months(months)), ticks(calendar.try_add_months(value, months)));
        if actual != (expected, calendar_expected) {
            mismatches.push(format!("{value:?} + {months} months: {actual:?}, .NET {:?}", (expected, calendar_expected)));
        }
    }
    for &(start, years, expected, calendar_expected) in net::ADD_YEARS {
        let value = DateTime::from_ticks(start);
        let actual = (ticks(value.try_add_years(years)), ticks(calendar.try_add_years(value, years)));
        if actual != (expected, calendar_expected) {
            mismatches.push(format!("{value:?} + {years} years: {actual:?}, .NET {:?}", (expected, calendar_expected)));
        }
    }
    for &(start, days, expected, calendar_expected) in net::ADD_DAYS {
        let value = DateTime::from_ticks(start);
        let actual = (ticks(value.try_add_days(days as f64)), ticks(calendar.try_add_days(value, days)));
        if actual != (expected, calendar_expected) {
            mismatches.push(format!("{value:?} + {days} days: {actual:?}, .NET {:?}", (expected, calendar_expected)));
        }
    }
    for &(start, days, expected) in net::ADD_FRACTIONAL_DAYS {
        let actual = DateTime::from_ticks(start).add_days(days).ticks();
        if actual != expected {
            mismatches.push(format!("{start} + {days} days: {actual}, .NET {expected}"));
        }
    }
    report("ADD_*", net::ADD_MONTHS.len() + net::ADD_YEARS.len() + net::ADD_DAYS.len() + net::ADD_FRACTIONAL_DAYS.len(), mismatches);
}

#[test]
fn parse_matches() {
    let cultures = cultures();
    let mut mismatches = Vec::new();
    for &(culture, text, expected) in net::PARSE {
        let targets: &[usize] = if culture == 2 { &[1, 2] } else { &[culture as usize] };
        for &target in targets {
            let actual = parsed(DateTime::parse(text, &cultures[target].date_time_format()).ok());
            if actual != expected {
                mismatches.push(format!("{:?} {text:?}: {actual:?}, .NET {expected:?}", net::CULTURES[target]));
            }
        }
    }
    report("PARSE", net::PARSE.len(), mismatches);
}

#[test]
fn parse_of_formatted_values_matches() {
    let cultures = cultures();
    let mut mismatches = Vec::new();
    for &(culture, ticks, format, text, expected) in net::PARSE_ROUND_TRIP {
        let culture = &cultures[culture as usize];
        let formatted = DateTime::from_ticks(ticks).to_string_format(format, culture);
        let actual = parsed(DateTime::parse(text, culture).ok());
        if formatted != text || actual != expected {
            mismatches.push(format!("{:?} {format:?} {text:?} (ours {formatted:?}): {actual:?}, .NET {expected:?}", culture.name()));
        }
    }
    report("PARSE_ROUND_TRIP", net::PARSE_ROUND_TRIP.len(), mismatches);
}

#[test]
fn parse_with_styles_matches() {
    let cultures = cultures();
    let mut mismatches = Vec::new();
    for &(text, styles, expected) in net::PARSE_STYLES {
        let styles = DateTimeStyles::from_bits_retain(styles);
        let actual = parsed(DateTime::parse_with_styles(text, &cultures[0], styles).ok());
        if actual != expected {
            mismatches.push(format!("{text:?} {styles:?}: {actual:?}, .NET {expected:?}"));
        }
    }
    report("PARSE_STYLES", net::PARSE_STYLES.len(), mismatches);
}

#[test]
fn parse_exact_matches() {
    let cultures = cultures();
    let mut mismatches = Vec::new();
    for &(culture, text, format, styles, expected) in net::PARSE_EXACT {
        let styles = DateTimeStyles::from_bits_retain(styles);
        let actual = parsed(DateTime::parse_exact_with_styles(text, format, &cultures[culture as usize], styles).ok());
        if actual != expected {
            mismatches.push(format!(
                "{:?} {text:?} {format:?} {styles:?}: {actual:?}, .NET {expected:?}",
                net::CULTURES[culture as usize]
            ));
        }
    }
    let formats = ["yyyy-MM-dd", "MM dd yyyy", "dd.MM.yyyy"];
    for &(text, expected) in net::PARSE_EXACT_MULTIPLE {
        let actual = parsed(DateTime::try_parse_exact_multiple(text, &formats, &cultures[0], DateTimeStyles::NONE));
        if actual != expected {
            mismatches.push(format!("{text:?} (several formats): {actual:?}, .NET {expected:?}"));
        }
    }
    report("PARSE_EXACT", net::PARSE_EXACT.len() + net::PARSE_EXACT_MULTIPLE.len(), mismatches);
}

#[test]
fn time_span_formatting_matches() {
    let mut mismatches = Vec::new();
    for &(culture, ticks, format, expected) in net::TIME_SPAN_FORMAT {
        // The decimal separator of every recorded culture is `.`, so no row depends on the culture.
        let actual = TimeSpan::from_ticks(ticks).try_to_string_format(format).ok();
        if actual.as_deref() != expected {
            mismatches.push(format!("{:?} {ticks} {format:?}: {actual:?}, .NET {expected:?}", net::CULTURES[culture as usize]));
        }
    }
    for &(ticks, days, hours, minutes, seconds, milliseconds, total_hours) in net::TIME_SPAN_PARTS {
        let value = TimeSpan::from_ticks(ticks);
        let actual = (value.days(), value.hours(), value.minutes(), value.seconds(), value.milliseconds(), value.total_hours());
        if actual != (days, hours, minutes, seconds, milliseconds, total_hours) {
            mismatches.push(format!("{ticks}: components {actual:?}"));
        }
    }
    report("TIME_SPAN_FORMAT", net::TIME_SPAN_FORMAT.len() + net::TIME_SPAN_PARTS.len(), mismatches);
}
