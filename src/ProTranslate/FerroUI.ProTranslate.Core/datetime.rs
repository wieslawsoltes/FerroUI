//! Typed date/time formatting backed by ICU calendars and Unicode CLDR data.
//!
//! Values carry an explicit UTC offset. Formatting never reads a host time zone,
//! or applies an IANA zone rule. The culture selects the display calendar; a
//! Unicode `u-ca` extension selects an explicit calendar. Custom forms accept
//! the source date tokens, with nanosecond fractions available as an extension.
use crate::{
    calendar::CalendarDate, date_data, Culture, FormatArgument, LocalizedFormat, TranslationError,
};
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DateTimeKind {
    Offset,
    Unspecified,
    Utc,
}
struct DateFormatContext<'a> {
    culture: &'a Culture,
    data: &'a DateData,
    cldr: bool,
    has_day: bool,
    calendar: &'a CalendarDate,
    kind: DateTimeKind,
}

pub(crate) struct DateData {
    pub locale: &'static str,
    pub short_date: &'static str,
    pub long_date: &'static str,
    pub short_time: &'static str,
    pub long_time: &'static str,
    pub month_day: &'static str,
    pub year_month: &'static str,
    pub months: [&'static str; 12],
    pub short_months: [&'static str; 12],
    pub standalone_months: [&'static str; 12],
    pub standalone_short_months: [&'static str; 12],
    pub days: [&'static str; 7],
    pub short_days: [&'static str; 7],
    pub am: &'static str,
    pub pm: &'static str,
    pub eras: [&'static str; 2],
    pub flexible_periods: &'static [(u16, u16, bool, &'static str)],
}
fn metadata(culture: &Culture) -> &'static DateData {
    if culture.is_invariant() {
        return &date_data::INVARIANT;
    }
    let mut current = Some(culture.clone());
    while let Some(culture) = current {
        if let Ok(index) =
            date_data::DATES.binary_search_by_key(&culture.name(), |data| data.locale)
        {
            return &date_data::DATES[index];
        }
        current = culture.parent();
    }
    &date_data::DATES[date_data::DATES
        .binary_search_by_key(&"en", |data| data.locale)
        .expect("CLDR includes en")]
}
/// A Gregorian instant with an explicit UTC offset, suitable for `FormatArgument`.
///
/// `O` uses seven fractional digits, as the source composite formatter does.
/// Custom `f`/`F` tokens support up to nine digits for Rust's nanosecond precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalizedDateTime(pub OffsetDateTime);
impl LocalizedDateTime {
    pub fn new(value: OffsetDateTime) -> Self {
        Self(value)
    }
    pub fn value(self) -> OffsetDateTime {
        self.0
    }
    pub fn from_unix_timestamp(
        seconds: i64,
        utc_offset_seconds: i32,
    ) -> Result<Self, TranslationError> {
        let instant = OffsetDateTime::from_unix_timestamp(seconds)
            .map_err(|e| TranslationError::Format(e.to_string()))?;
        let offset = UtcOffset::from_whole_seconds(utc_offset_seconds)
            .map_err(|e| TranslationError::Format(e.to_string()))?;
        Ok(Self(instant.to_offset(offset)))
    }
    pub fn format(&self, format: &str, culture: &Culture) -> Result<String, TranslationError> {
        self.format_kind(format, culture, DateTimeKind::Offset)
    }
    fn format_kind(
        &self,
        format: &str,
        culture: &Culture,
        kind: DateTimeKind,
    ) -> Result<String, TranslationError> {
        let data = metadata(culture);
        if format.is_empty() {
            let general = self.format_kind("G", culture, kind)?;
            return Ok(if kind == DateTimeKind::Offset {
                format!("{general} {}", self.offset(true, false))
            } else {
                general
            });
        }
        if format.chars().count() != 1 {
            return self.pattern(format, culture, data, false, kind);
        }
        let standard = format.chars().next().expect("format is nonempty");
        let cldr = |pattern| self.pattern(pattern, culture, data, true, kind);
        let date = |specifier, pattern| -> Result<String, TranslationError> {
            let calendar = CalendarDate::new(self.0, culture)?;
            if calendar.is_gregorian() {
                cldr(pattern)
            } else {
                calendar.standard(specifier)
            }
        };
        let join =
            |date_specifier, date_pattern, time_pattern| -> Result<String, TranslationError> {
                Ok(format!(
                    "{} {}",
                    date(date_specifier, date_pattern)?,
                    cldr(time_pattern)?
                ))
            };
        let invariant = Culture::invariant();
        let invariant_data = metadata(&invariant);
        match standard {
            'd' => date('d', data.short_date),
            'D' => date('D', data.long_date),
            't' => cldr(data.short_time),
            'T' => cldr(data.long_time),
            'f' => join('D', data.long_date, data.short_time),
            'F' => join('D', data.long_date, data.long_time),
            'g' => join('d', data.short_date, data.short_time),
            'G' => join('d', data.short_date, data.long_time),
            'M' | 'm' => date('M', data.month_day),
            'Y' | 'y' => date('Y', data.year_month),
            'O' | 'o' => self.pattern(
                if kind == DateTimeKind::Offset {
                    "yyyy-MM-dd'T'HH:mm:ss.fffffffzzz"
                } else {
                    "yyyy-MM-dd'T'HH:mm:ss.fffffffK"
                },
                &invariant,
                invariant_data,
                false,
                kind,
            ),
            's' => self.pattern(
                "yyyy-MM-dd'T'HH:mm:ss",
                &invariant,
                invariant_data,
                false,
                kind,
            ),
            'u' => Self(self.0.to_offset(UtcOffset::UTC)).pattern(
                "yyyy-MM-dd HH:mm:ss'Z'",
                &invariant,
                invariant_data,
                false,
                kind,
            ),
            'U' if kind == DateTimeKind::Utc => self.format_kind("F", culture, kind),
            'U' => Err(TranslationError::Format(
                "Universal full formatting requires an explicitly UTC date/time".into(),
            )),
            'r' | 'R' => Self(self.0.to_offset(UtcOffset::UTC)).pattern(
                "ddd, dd MMM yyyy HH:mm:ss 'GMT'",
                &invariant,
                invariant_data,
                false,
                kind,
            ),
            _ => Err(TranslationError::Format(format!(
                "Unsupported standard date format '{standard}'"
            ))),
        }
    }
    fn pattern(
        &self,
        pattern: &str,
        culture: &Culture,
        data: &DateData,
        cldr: bool,
        kind: DateTimeKind,
    ) -> Result<String, TranslationError> {
        let tokens = tokenize(pattern, cldr)?;
        let has_day = tokens
            .iter()
            .any(|token| matches!(token, Token::Field('d', 1 | 2)));
        let calendar = CalendarDate::new(self.0, culture)?;
        let context = DateFormatContext {
            culture,
            data,
            cldr,
            has_day,
            calendar: &calendar,
            kind,
        };
        let mut output = String::new();
        for token in tokens {
            match token {
                Token::Literal(value) => output.push_str(&value),
                Token::Field(field, count) => {
                    let count = if cldr && field == 'y' && count < 4 {
                        4
                    } else {
                        count
                    };
                    let formatted = self.field(field, count, &context)?;
                    if !cldr && field == 'F' && formatted.is_empty() && output.ends_with('.') {
                        output.pop();
                    }
                    output.push_str(&formatted);
                }
            }
        }
        Ok(output)
    }
    fn field(
        &self,
        field: char,
        count: usize,
        context: &DateFormatContext<'_>,
    ) -> Result<String, TranslationError> {
        let DateFormatContext {
            culture,
            data,
            cldr,
            has_day,
            calendar,
            kind,
        } = *context;
        let date = self.0;
        let number = |value: i64, digits: usize| {
            FormatArgument::Integer(value).format(&format!("D{digits}"), culture)
        };
        let month = date.month() as usize - 1;
        let weekday = date.weekday().number_days_from_sunday() as usize;
        match field {
            'y' => {
                let year = calendar.year();
                let short = if cldr { count == 2 } else { count <= 2 };
                number(
                    if short { year % 100 } else { year } as i64,
                    if count == 1 { 0 } else { count },
                )
            }
            'M' | 'L' if cldr || field == 'M' => {
                if count < 3 {
                    number(calendar.month() as i64, count)
                } else {
                    let standalone = field == 'L' || (!cldr && !has_day);
                    if !calendar.is_gregorian() {
                        return calendar.month_name(count == 3, standalone);
                    }
                    let name = if count == 3 {
                        if standalone {
                            data.standalone_short_months[month]
                        } else {
                            data.short_months[month]
                        }
                    } else if standalone {
                        data.standalone_months[month]
                    } else {
                        data.months[month]
                    };
                    Ok(if cldr && count == 5 {
                        name.chars().take(1).collect()
                    } else {
                        name.into()
                    })
                }
            }
            'd' if cldr || count < 3 => number(calendar.day() as i64, count.min(2)),
            'd' => Ok(if count == 3 {
                data.short_days[weekday]
            } else {
                data.days[weekday]
            }
            .into()),
            'E' | 'e' | 'c' if cldr => Ok(if count < 4 {
                data.short_days[weekday].into()
            } else if count == 5 {
                data.days[weekday].chars().take(1).collect()
            } else {
                data.days[weekday].into()
            }),
            'H' => number(date.hour() as i64, count.min(2)),
            'h' => number(
                if date.hour().is_multiple_of(12) {
                    12
                } else {
                    date.hour() % 12
                } as i64,
                count.min(2),
            ),
            'K' if cldr => number((date.hour() % 12) as i64, count),
            'k' if cldr => number(
                if date.hour() == 0 { 24 } else { date.hour() } as i64,
                count,
            ),
            'm' => number(date.minute() as i64, count.min(2)),
            's' => number(date.second() as i64, count.min(2)),
            'a' if cldr => Ok(if date.hour() < 12 { data.am } else { data.pm }.into()),
            'b' | 'B' if cldr => {
                let minute = u16::from(date.hour()) * 60 + u16::from(date.minute());
                let period = data
                    .flexible_periods
                    .iter()
                    .find(|(start, end, exact, _)| {
                        if *exact {
                            minute == *start && date.second() == 0 && date.nanosecond() == 0
                        } else if field == 'b' {
                            false
                        } else if start < end {
                            minute >= *start && minute < *end
                        } else {
                            minute >= *start || minute < *end
                        }
                    })
                    .map(|(_, _, _, name)| *name)
                    .unwrap_or(if date.hour() < 12 { data.am } else { data.pm });
                Ok(period.into())
            }
            't' if !cldr => {
                let period = if date.hour() < 12 { data.am } else { data.pm };
                Ok(if count == 1 {
                    period.chars().take(1).collect()
                } else {
                    period.into()
                })
            }
            'f' | 'F' if !cldr => {
                if count > 9 {
                    return Err(TranslationError::Format(
                        "Fractional-second precision must be at most nine".into(),
                    ));
                }
                let digits = format!("{:09}", date.nanosecond());
                let value = &digits[..count];
                Ok(if field == 'F' {
                    value.trim_end_matches('0')
                } else {
                    value
                }
                .into())
            }
            'S' if cldr => {
                if count > 9 {
                    return Err(TranslationError::Format(
                        "Fractional-second precision must be at most nine".into(),
                    ));
                }
                Ok(format!("{:09}", date.nanosecond())[..count].into())
            }
            'z' | 'Z' | 'O' | 'v' | 'V' | 'X' | 'x' if cldr => Ok(self.offset(true, false)),
            'z' if !cldr => {
                if kind == DateTimeKind::Unspecified {
                    return Err(TranslationError::Format("An unspecified date/time has no UTC offset; supply an explicit offset value".into()));
                }
                let offset = date.offset().whole_seconds();
                let sign = if offset < 0 { '-' } else { '+' };
                let hours = offset.unsigned_abs() / 3600;
                Ok(if count == 1 {
                    format!("{sign}{hours}")
                } else if count == 2 {
                    format!("{sign}{hours:02}")
                } else {
                    self.offset(true, false)
                })
            }
            'K' if !cldr => Ok(match kind {
                DateTimeKind::Offset => self.offset(true, false),
                DateTimeKind::Utc => "Z".into(),
                DateTimeKind::Unspecified => String::new(),
            }
            .repeat(count)),
            'G' if cldr => {
                if calendar.is_gregorian() {
                    Ok(data.eras[usize::from(date.year() > 0)].into())
                } else {
                    calendar.era_name()
                }
            }
            'g' if !cldr => {
                if calendar.is_gregorian() {
                    Ok(data.eras[usize::from(date.year() > 0)].into())
                } else {
                    calendar.era_name()
                }
            }
            '/' if !cldr => Ok(separator(data.short_date, '/').repeat(count)),
            ':' if !cldr => Ok(separator(data.short_time, ':').repeat(count)),
            _ if !cldr => Ok(std::iter::repeat_n(field, count).collect()),
            _ => Err(TranslationError::Format(format!(
                "Unsupported date token '{field}'"
            ))),
        }
    }
    fn offset(&self, colon: bool, utc_z: bool) -> String {
        let seconds = self.0.offset().whole_seconds();
        if seconds == 0 && utc_z {
            return "Z".into();
        }
        let sign = if seconds < 0 { '-' } else { '+' };
        let total = seconds.unsigned_abs();
        format!(
            "{sign}{:02}{}{:02}",
            total / 3600,
            if colon { ":" } else { "" },
            total / 60 % 60
        )
    }
}
impl LocalizedFormat for LocalizedDateTime {
    fn format(&self, format: &str, culture: &Culture) -> Result<String, TranslationError> {
        self.format(format, culture)
    }
}
impl From<OffsetDateTime> for LocalizedDateTime {
    fn from(value: OffsetDateTime) -> Self {
        Self(value)
    }
}
impl From<LocalizedDateTime> for FormatArgument {
    fn from(value: LocalizedDateTime) -> Self {
        Self::DateTime(value)
    }
}
impl From<OffsetDateTime> for FormatArgument {
    fn from(value: OffsetDateTime) -> Self {
        Self::DateTime(LocalizedDateTime(value))
    }
}
/// A wall-clock date/time with no inferred machine time zone. `new` preserves an
/// unspecified value; `utc` explicitly identifies the value as UTC.
///
/// Round-trip `O` and custom `K` retain that distinction. `z` and universal `U`
/// require an explicit UTC or offset value instead of assuming a host time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalizedNaiveDateTime {
    value: PrimitiveDateTime,
    utc: bool,
}
impl LocalizedNaiveDateTime {
    pub fn new(value: PrimitiveDateTime) -> Self {
        Self { value, utc: false }
    }
    pub fn utc(value: PrimitiveDateTime) -> Self {
        Self { value, utc: true }
    }
    pub fn value(self) -> PrimitiveDateTime {
        self.value
    }
    pub fn is_utc(self) -> bool {
        self.utc
    }
    pub fn format(&self, format: &str, culture: &Culture) -> Result<String, TranslationError> {
        LocalizedDateTime(self.value.assume_utc()).format_kind(
            format,
            culture,
            if self.utc {
                DateTimeKind::Utc
            } else {
                DateTimeKind::Unspecified
            },
        )
    }
}
impl LocalizedFormat for LocalizedNaiveDateTime {
    fn format(&self, format: &str, culture: &Culture) -> Result<String, TranslationError> {
        self.format(format, culture)
    }
}
impl From<PrimitiveDateTime> for LocalizedNaiveDateTime {
    fn from(value: PrimitiveDateTime) -> Self {
        Self::new(value)
    }
}
impl From<LocalizedNaiveDateTime> for FormatArgument {
    fn from(value: LocalizedNaiveDateTime) -> Self {
        Self::NaiveDateTime(value)
    }
}
impl From<PrimitiveDateTime> for FormatArgument {
    fn from(value: PrimitiveDateTime) -> Self {
        Self::NaiveDateTime(value.into())
    }
}
fn separator(pattern: &str, fallback: char) -> String {
    pattern
        .chars()
        .find(|ch| matches!(ch, '/' | '.' | '-' | ':'))
        .unwrap_or(fallback)
        .to_string()
}
enum Token {
    Literal(String),
    Field(char, usize),
}
fn tokenize(pattern: &str, cldr: bool) -> Result<Vec<Token>, TranslationError> {
    let mut result = Vec::new();
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && !cldr {
            result.push(Token::Literal(
                chars
                    .next()
                    .ok_or_else(|| TranslationError::Format("Trailing date-format escape".into()))?
                    .to_string(),
            ));
        } else if ch == '\'' || (ch == '"' && !cldr) {
            let quote = ch;
            let mut literal = String::new();
            let mut closed = false;
            if cldr && chars.peek() == Some(&quote) {
                chars.next();
                literal.push(quote);
                closed = true;
            } else {
                while let Some(ch) = chars.next() {
                    if ch == quote {
                        if cldr && chars.peek() == Some(&quote) {
                            chars.next();
                            literal.push(quote);
                        } else {
                            closed = true;
                            break;
                        }
                    } else if ch == '\\' && !cldr {
                        literal.push(chars.next().ok_or_else(|| {
                            TranslationError::Format("Trailing quoted date escape".into())
                        })?);
                    } else {
                        literal.push(ch);
                    }
                }
            }
            if !closed {
                return Err(TranslationError::Format(
                    "Unclosed date-format quote".into(),
                ));
            }
            result.push(Token::Literal(literal));
        } else if ch == '%' && !cldr {
            let field = chars
                .next()
                .ok_or_else(|| TranslationError::Format("Trailing date-format percent".into()))?;
            if matches!(field, '%' | '\'' | '"' | '\\') {
                return Err(TranslationError::Format(
                    "Invalid percent date escape".into(),
                ));
            }
            result.push(Token::Field(field, 1));
        } else if ch.is_ascii_alphabetic() || (!cldr && matches!(ch, '/' | ':')) {
            let mut count = 1;
            while chars.peek() == Some(&ch) {
                chars.next();
                count += 1;
            }
            if count > 1024 {
                return Err(TranslationError::Format(
                    "Date token length exceeds 1024".into(),
                ));
            }
            result.push(Token::Field(ch, count));
        } else {
            result.push(Token::Literal(ch.to_string()));
        }
    }
    Ok(result)
}
