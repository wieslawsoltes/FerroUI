//! A culture data provider for tests: the conventions of `en`, `en-US` and
//! `en-GB` (the cultures the control tests name).
//!
//! The values are the ones .NET 10.0.5 reports on macOS, where it takes
//! culture data from ICU (`date_time_net_tests.rs` checks them against the
//! recorded output). Note the narrow no-break space (U+202F) before the
//! designator in the time patterns of `en` and `en-US`.

use super::{CalendarWeekRule, CultureTypes, DateTimeFormatInfo, DayOfWeek, ICultureDataProvider, NumberFormatInfo};
use crate::FerroLocator;
use std::rc::Rc;

/// Supplies `en`, `en-US` and `en-GB`; every other culture has no data.
pub struct TestCultureDataProvider {
    en_us: Rc<DateTimeFormatInfo>,
    en_gb: Rc<DateTimeFormatInfo>,
    en_numbers: Rc<NumberFormatInfo>,
    en_us_numbers: Rc<NumberFormatInfo>,
    en_gb_numbers: Rc<NumberFormatInfo>,
}

impl TestCultureDataProvider {
    /// Creates the provider.
    pub fn new() -> TestCultureDataProvider {
        // `en` and `en-US` have the same date and time conventions.
        let mut en_us = DateTimeFormatInfo::new();
        en_us.set_short_date_pattern("M/d/yyyy");
        en_us.set_long_date_pattern("dddd, MMMM d, yyyy");
        en_us.set_short_time_pattern("h:mm\u{202F}tt");
        en_us.set_long_time_pattern("h:mm:ss\u{202F}tt");
        en_us.set_month_day_pattern("MMMM d");
        en_us.set_year_month_pattern("MMMM yyyy");
        en_us.set_era_name("AD");

        let mut en_gb = DateTimeFormatInfo::new();
        en_gb.set_short_date_pattern("dd/MM/yyyy");
        en_gb.set_long_date_pattern("dddd, d MMMM yyyy");
        en_gb.set_month_day_pattern("d MMMM");
        en_gb.set_year_month_pattern("MMMM yyyy");
        en_gb.set_am_designator("am");
        en_gb.set_pm_designator("pm");
        en_gb.set_era_name("AD");
        en_gb.set_first_day_of_week(DayOfWeek::Monday);
        en_gb.set_calendar_week_rule(CalendarWeekRule::FirstFourDayWeek);

        // number format: owned by numeric agent (the values are the .NET ones; separators,
        // group sizes, signs, the NaN, percent and per mille symbols equal the invariant ones).
        let english_numbers = || {
            NumberFormatInfo::new()
                .with_currency_negative_pattern(1)
                .with_number_decimal_digits(3)
                .with_percent_decimal_digits(3)
                .with_percent_negative_pattern(1)
                .with_percent_positive_pattern(1)
                .with_positive_infinity_symbol("\u{221E}")
                .with_negative_infinity_symbol("-\u{221E}")
        };

        TestCultureDataProvider {
            en_us: Rc::new(en_us),
            en_gb: Rc::new(en_gb),
            // The neutral culture has the generic currency sign.
            en_numbers: Rc::new(english_numbers()),
            en_us_numbers: Rc::new(english_numbers().with_currency_symbol("$")),
            en_gb_numbers: Rc::new(english_numbers().with_currency_symbol("\u{00A3}")),
        }
    }

    /// Registers a new provider with the current (mutable) service locator.
    /// Enter a locator scope first to keep the registration local to a test.
    pub fn register() {
        let provider: Rc<dyn ICultureDataProvider> = Rc::new(TestCultureDataProvider::new());
        FerroLocator::current_mutable().bind::<dyn ICultureDataProvider>().to_constant(provider);
    }
}

impl Default for TestCultureDataProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ICultureDataProvider for TestCultureDataProvider {
    fn get_date_time_format(&self, culture_name: &str) -> Option<Rc<DateTimeFormatInfo>> {
        if culture_name.eq_ignore_ascii_case("en") || culture_name.eq_ignore_ascii_case("en-US") {
            Some(Rc::clone(&self.en_us))
        } else if culture_name.eq_ignore_ascii_case("en-GB") {
            Some(Rc::clone(&self.en_gb))
        } else {
            None
        }
    }

    // number format: owned by numeric agent
    fn get_number_format(&self, culture_name: &str) -> Option<Rc<NumberFormatInfo>> {
        if culture_name.eq_ignore_ascii_case("en") {
            Some(Rc::clone(&self.en_numbers))
        } else if culture_name.eq_ignore_ascii_case("en-US") {
            Some(Rc::clone(&self.en_us_numbers))
        } else if culture_name.eq_ignore_ascii_case("en-GB") {
            Some(Rc::clone(&self.en_gb_numbers))
        } else {
            None
        }
    }

    fn get_culture_names(&self, types: CultureTypes) -> Vec<String> {
        let mut names = Vec::new();
        if types.contains(CultureTypes::NEUTRAL_CULTURES) {
            names.push("en".to_string());
        }
        if types.contains(CultureTypes::SPECIFIC_CULTURES) {
            names.push("en-GB".to_string());
            names.push("en-US".to_string());
        }
        names
    }
}
