//! Typed Gregorian date/time formatting backed by Unicode CLDR data.
//!
//! Values carry an explicit UTC offset. Formatting never reads a host time zone,
//! applies an IANA zone rule, or changes calendar. Standard localized forms use
//! CLDR Gregorian conventions; custom forms accept the common .NET date tokens.
use crate::{date_data, Culture, FormatArgument, LocalizedFormat, TranslationError};
use time::{OffsetDateTime, UtcOffset};

pub(crate) struct DateData {
    pub locale: &'static str,
    pub short_date: &'static str,
    pub short_date_numbers: &'static str,
    pub long_date: &'static str,
    pub short_time: &'static str,
    pub long_time: &'static str,
    pub join: &'static str,
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
}
fn metadata(culture: &Culture) -> &'static DateData {
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
        let data = metadata(culture);
        let format = if format.is_empty() { "G" } else { format };
        if format.chars().count() != 1 {
            return self.pattern(format, culture, data, false);
        }
        let standard = format.chars().next().expect("format is nonempty");
        let cldr = |pattern| self.pattern(pattern, culture, data, true);
        let join = |date, time| -> Result<String, TranslationError> {
            Ok(data
                .join
                .replace("{1}", &cldr(date)?)
                .replace("{0}", &cldr(time)?))
        };
        match standard {
            'd' => cldr(data.short_date),
            'D' => cldr(data.long_date),
            't' => cldr(data.short_time),
            'T' => cldr(data.long_time),
            'f' => join(data.long_date, data.short_time),
            'F' => join(data.long_date, data.long_time),
            'g' => join(data.short_date, data.short_time),
            'G' => join(data.short_date, data.long_time),
            'M' | 'm' => cldr(data.month_day),
            'Y' | 'y' => cldr(data.year_month),
            'O' | 'o' => self.pattern(
                "yyyy-MM-dd'T'HH:mm:ss.fffffffzzz",
                &Culture::invariant(),
                metadata(&Culture::invariant()),
                false,
            ),
            's' => self.pattern(
                "yyyy-MM-dd'T'HH:mm:ss",
                &Culture::invariant(),
                metadata(&Culture::invariant()),
                false,
            ),
            'u' => Self(self.0.to_offset(UtcOffset::UTC)).pattern(
                "yyyy-MM-dd HH:mm:ss'Z'",
                &Culture::invariant(),
                metadata(&Culture::invariant()),
                false,
            ),
            'U' => Self(self.0.to_offset(UtcOffset::UTC)).format("F", culture),
            'r' | 'R' => Self(self.0.to_offset(UtcOffset::UTC)).pattern(
                "ddd, dd MMM yyyy HH:mm:ss 'GMT'",
                &Culture::invariant(),
                metadata(&Culture::invariant()),
                false,
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
    ) -> Result<String, TranslationError> {
        let adapted;
        let pattern =
            if cldr && pattern == data.short_date && data.short_date_numbers == "M=romanlow" {
                let roman = [
                    "i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x", "xi", "xii",
                ][self.0.month() as usize - 1];
                adapted = pattern.replace('M', &format!("'{roman}'"));
                &adapted
            } else {
                pattern
            };
        let tokens = tokenize(pattern, cldr)?;
        let has_day = tokens
            .iter()
            .any(|token| matches!(token, Token::Field('d', 1 | 2)));
        let mut output = String::new();
        for token in tokens {
            match token {
                Token::Literal(value) => output.push_str(&value),
                Token::Field(field, count) => {
                    output.push_str(&self.field(field, count, culture, data, cldr, has_day)?)
                }
            }
        }
        Ok(output)
    }
    fn field(
        &self,
        field: char,
        count: usize,
        culture: &Culture,
        data: &DateData,
        cldr: bool,
        has_day: bool,
    ) -> Result<String, TranslationError> {
        let date = self.0;
        let number = |value: i64, digits: usize| {
            FormatArgument::Integer(value).format(&format!("D{digits}"), culture)
        };
        let month = date.month() as usize - 1;
        let weekday = date.weekday().number_days_from_sunday() as usize;
        match field {
            'y' => {
                let year = if date.year() <= 0 {
                    1 - date.year()
                } else {
                    date.year()
                };
                number(
                    if count == 2 { year % 100 } else { year } as i64,
                    if count == 1 { 0 } else { count },
                )
            }
            'M' | 'L' if cldr || field == 'M' => {
                if count < 3 {
                    number(month as i64 + 1, count)
                } else {
                    let standalone = field == 'L' || (!cldr && !has_day);
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
            'd' if cldr || count < 3 => number(date.day() as i64, count),
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
            'H' => number(date.hour() as i64, count),
            'h' => number(
                if date.hour().is_multiple_of(12) {
                    12
                } else {
                    date.hour() % 12
                } as i64,
                count,
            ),
            'K' if cldr => number((date.hour() % 12) as i64, count),
            'k' if cldr => number(
                if date.hour() == 0 { 24 } else { date.hour() } as i64,
                count,
            ),
            'm' => number(date.minute() as i64, count),
            's' => number(date.second() as i64, count),
            'a' | 'b' | 'B' if cldr => Ok(if date.hour() < 12 { data.am } else { data.pm }.into()),
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
            'K' if !cldr => Ok(self.offset(true, true)),
            'G' if cldr => Ok(data.eras[usize::from(date.year() > 0)].into()),
            'g' if !cldr => Ok(data.eras[usize::from(date.year() > 0)].into()),
            '/' if !cldr => Ok(separator(data.short_date, '/')),
            ':' if !cldr => Ok(separator(data.short_time, ':')),
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
            if chars.peek() == Some(&quote) {
                chars.next();
                literal.push(quote);
                closed = true;
            } else {
                while let Some(ch) = chars.next() {
                    if ch == quote {
                        if chars.peek() == Some(&quote) {
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
            if field == '%' {
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
