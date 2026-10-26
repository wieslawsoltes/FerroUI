use crate::{Culture, TranslationError};
use icu_calendar::{
    preferences::{CalendarAlgorithm, CalendarPreferences, HijriCalendarAlgorithm},
    AnyCalendar, AnyCalendarKind, Date,
};
use icu_datetime::{
    fieldsets, options::YearStyle, parts, DateTimeFormatter, DateTimeFormatterPreferences,
};
use std::fmt;
use time::OffsetDateTime;
use writeable::{Part, PartsWrite, Writeable};

/// Conversion and localized calendar fields are supplied by ICU's compiled
/// calendar data, including era boundaries and leap-month calendars.
pub(crate) struct CalendarDate {
    date: Date<AnyCalendar>,
    kind: AnyCalendarKind,
    preferences: DateTimeFormatterPreferences,
}
impl CalendarDate {
    pub fn new(value: OffsetDateTime, culture: &Culture) -> Result<Self, TranslationError> {
        let locale = culture.locale();
        let mut preferences = CalendarPreferences::from_locale_strict(&locale)
            .map_err(|_| TranslationError::Format("Unsupported calendar preference".into()))?;
        // Preserve the source runtime's Saudi Arabic default. Newer CLDR data
        // prefers Gregorian here; explicit `u-ca` preferences still take priority.
        if preferences.calendar_algorithm.is_none()
            && locale.id.language.as_str() == "ar"
            && locale
                .id
                .region
                .is_some_and(|region| region.as_str() == "SA")
        {
            preferences.calendar_algorithm = Some(CalendarAlgorithm::Hijri(Some(
                HijriCalendarAlgorithm::Umalqura,
            )));
        }
        let mut formatting_preferences: DateTimeFormatterPreferences = locale.into();
        formatting_preferences.calendar_algorithm = preferences.calendar_algorithm;
        let kind = if culture.is_invariant() {
            AnyCalendarKind::Gregorian
        } else {
            AnyCalendarKind::try_new(preferences).map_err(calendar_error)?
        };
        let date = Date::try_new_iso(value.year(), value.month() as u8, value.day())
            .map_err(calendar_error)?
            .to_calendar(AnyCalendar::new(kind));
        Ok(Self {
            date,
            kind,
            preferences: formatting_preferences,
        })
    }
    pub fn is_gregorian(&self) -> bool {
        matches!(self.kind, AnyCalendarKind::Gregorian | AnyCalendarKind::Iso)
    }
    pub fn year(&self) -> i32 {
        self.date.year().era_year_or_related_iso()
    }
    pub fn month(&self) -> u8 {
        self.date.month().ordinal
    }
    pub fn day(&self) -> u8 {
        self.date.day_of_month().0
    }
    pub fn standard(&self, specifier: char) -> Result<String, TranslationError> {
        macro_rules! format {
            ($fields:expr) => {{
                let formatter = DateTimeFormatter::try_new(self.preferences, $fields)
                    .map_err(calendar_error)?;
                Ok(formatter.format(&self.date).to_string())
            }};
        }
        match specifier {
            'd' => format!(fieldsets::YMD::short().with_year_style(YearStyle::Full)),
            'D' => format!(fieldsets::YMDE::long().with_year_style(YearStyle::Full)),
            'M' | 'm' => format!(fieldsets::MD::long()),
            'Y' | 'y' => format!(fieldsets::YM::long().with_year_style(YearStyle::Full)),
            _ => Err(TranslationError::Format(
                "Unsupported calendar date format".into(),
            )),
        }
    }
    pub fn month_name(
        &self,
        abbreviated: bool,
        standalone: bool,
    ) -> Result<String, TranslationError> {
        macro_rules! month {
            ($fields:expr) => {{
                let formatter = DateTimeFormatter::try_new(self.preferences, $fields)
                    .map_err(calendar_error)?;
                let mut output = FieldWriter::new(parts::MONTH);
                formatter
                    .format(&self.date)
                    .write_to_parts(&mut output)
                    .map_err(calendar_error)?;
                Ok(output.text)
            }};
        }
        match (abbreviated, standalone) {
            (true, true) => month!(fieldsets::M::medium()),
            (false, true) => month!(fieldsets::M::long()),
            (true, false) => month!(fieldsets::MD::medium()),
            (false, false) => month!(fieldsets::MD::long()),
        }
    }
    pub fn era_name(&self) -> Result<String, TranslationError> {
        let formatter = DateTimeFormatter::try_new(
            self.preferences,
            fieldsets::YMD::long().with_year_style(YearStyle::WithEra),
        )
        .map_err(calendar_error)?;
        let mut output = FieldWriter::new(parts::ERA);
        formatter
            .format(&self.date)
            .write_to_parts(&mut output)
            .map_err(calendar_error)?;
        Ok(output.text)
    }
}
fn calendar_error(error: impl fmt::Display) -> TranslationError {
    TranslationError::Format(format!("Calendar formatting failed: {error}"))
}
struct FieldWriter {
    wanted: Part,
    active: bool,
    text: String,
}
impl FieldWriter {
    fn new(wanted: Part) -> Self {
        Self {
            wanted,
            active: false,
            text: String::new(),
        }
    }
}
impl fmt::Write for FieldWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.active {
            self.text.push_str(value);
        }
        Ok(())
    }
}
impl PartsWrite for FieldWriter {
    type SubPartsWrite = Self;
    fn with_part(
        &mut self,
        part: Part,
        mut write: impl FnMut(&mut Self) -> fmt::Result,
    ) -> fmt::Result {
        let active = self.active;
        self.active |= part == self.wanted;
        let result = write(self);
        self.active = active;
        result
    }
}
