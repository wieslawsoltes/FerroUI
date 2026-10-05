//! The engine behind date and time formatting and parsing: the standard and
//! custom .NET format strings, exact parsing against a format and the
//! free-form parser.
//!
//! Formatting and exact parsing follow the documented meaning of each format
//! specifier. The free-form parser accepts the common notations (numeric
//! dates in the order of the culture, month names, ISO 8601, times with or
//! without designator, offsets); it is not the heuristic parser of .NET and
//! rejects exotic inputs that one accepts.

use super::{DateTime, DateTimeFormatInfo, DateTimeKind, DateTimeStyles, DayOfWeek, FormatError, TimeZoneInfo};
use crate::animation::TimeSpan;
use std::borrow::Cow;

const ROUND_TRIP_PATTERN: &str = "yyyy'-'MM'-'dd'T'HH':'mm':'ss.fffffffK";
/// Years 00..=49 of a two-digit year are in the 2000s, 50..=99 in the 1900s
/// (the `TwoDigitYearMax` of the Gregorian calendar is 2049).
const TWO_DIGIT_YEAR_MAX: i32 = 2049;

/// The outcome of parsing: the clock time as written and, when the text
/// states one, its offset from UTC.
pub(crate) struct Parsed {
    /// The date and time as written, of unspecified kind.
    pub date_time: DateTime,
    /// The offset stated by the text (`Z` is zero).
    pub offset: Option<TimeSpan>,
    /// Whether the offset was stated with the UTC designator `Z`.
    pub utc_designator: bool,
}

fn bad_format() -> FormatError {
    FormatError::new("Input string was not in a correct format.")
}

fn not_recognized(text: &str) -> FormatError {
    FormatError::from_string(format!("String '{text}' was not recognized as a valid DateTime."))
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Formats the clock time `value`. `offset` is the offset of a
/// `DateTimeOffset`; `None` formats a plain `DateTime`.
pub(crate) fn format(
    value: DateTime,
    offset: Option<TimeSpan>,
    format: &str,
    info: &DateTimeFormatInfo,
) -> Result<String, FormatError> {
    let mut value = value;
    let mut offset = offset;
    let mut chars = format.chars();
    let (first, second) = (chars.next(), chars.next());

    let Some(standard) = first.filter(|_| second.is_none()) else {
        let pattern: Cow<'_, str> = if !format.is_empty() {
            Cow::Borrowed(format)
        } else if offset.is_some() {
            Cow::Owned(format!("{} {} zzz", info.short_date_pattern(), info.long_time_pattern()))
        } else {
            Cow::Owned(format!("{} {}", info.short_date_pattern(), info.long_time_pattern()))
        };
        return format_custom(value, offset, &pattern, info);
    };

    let invariant = DateTimeFormatInfo::invariant_info();
    // The formats that state UTC print the UTC time of a value with an offset.
    fn to_utc(value: &mut DateTime, offset: &mut Option<TimeSpan>) {
        if let Some(current) = *offset {
            *value = DateTime::from_ticks(value.ticks() - current.ticks());
            *offset = Some(TimeSpan::ZERO);
        }
    }

    let (pattern, info): (Cow<'_, str>, &DateTimeFormatInfo) = match standard {
        'd' => (Cow::Borrowed(info.short_date_pattern()), info),
        'D' => (Cow::Borrowed(info.long_date_pattern()), info),
        'f' => (Cow::Owned(format!("{} {}", info.long_date_pattern(), info.short_time_pattern())), info),
        'F' => (info.full_date_time_pattern(), info),
        'g' => (Cow::Owned(format!("{} {}", info.short_date_pattern(), info.short_time_pattern())), info),
        'G' => (Cow::Owned(format!("{} {}", info.short_date_pattern(), info.long_time_pattern())), info),
        'm' | 'M' => (Cow::Borrowed(info.month_day_pattern()), info),
        'o' | 'O' => (Cow::Borrowed(ROUND_TRIP_PATTERN), &*invariant),
        'r' | 'R' => {
            to_utc(&mut value, &mut offset);
            (Cow::Borrowed(invariant.rfc1123_pattern()), &*invariant)
        }
        's' => (Cow::Borrowed(invariant.sortable_date_time_pattern()), &*invariant),
        't' => (Cow::Borrowed(info.short_time_pattern()), info),
        'T' => (Cow::Borrowed(info.long_time_pattern()), info),
        'u' => {
            to_utc(&mut value, &mut offset);
            (Cow::Borrowed(invariant.universal_sortable_date_time_pattern()), &*invariant)
        }
        'U' => {
            if offset.is_some() {
                return Err(bad_format());
            }
            value = value.to_universal_time();
            (info.full_date_time_pattern(), info)
        }
        'y' | 'Y' => (Cow::Borrowed(info.year_month_pattern()), info),
        _ => return Err(bad_format()),
    };
    format_custom(value, offset, &pattern, info)
}

fn push_number(out: &mut String, value: i64, min_digits: usize) {
    use std::fmt::Write;
    let _ = write!(out, "{value:0min_digits$}");
}

fn push_offset(out: &mut String, offset: TimeSpan, count: usize) {
    let minutes = offset.ticks() / TimeSpan::TICKS_PER_MINUTE;
    out.push(if minutes < 0 { '-' } else { '+' });
    let minutes = minutes.abs();
    match count {
        1 => push_number(out, minutes / 60, 1),
        2 => push_number(out, minutes / 60, 2),
        _ => {
            push_number(out, minutes / 60, 2);
            out.push(':');
            push_number(out, minutes % 60, 2);
        }
    }
}

struct FormatContext<'a> {
    value: DateTime,
    offset: Option<TimeSpan>,
    info: &'a DateTimeFormatInfo,
    year: i32,
    month: i32,
    day: i32,
}

impl FormatContext<'_> {
    /// The offset the `z` specifiers print.
    fn zone_offset(&self) -> TimeSpan {
        match self.offset {
            Some(offset) => offset,
            None if self.value.kind() == DateTimeKind::Utc => TimeSpan::ZERO,
            None => TimeZoneInfo::get_utc_offset_of_local_time(self.value),
        }
    }

    /// Writes what `count` repetitions of the specifier `specifier` stand
    /// for; `Ok(false)` when the character is no specifier.
    fn write(&self, out: &mut String, specifier: u8, count: usize) -> Result<bool, FormatError> {
        let value = self.value;
        let info = self.info;
        let two = |count: usize| if count >= 2 { 2 } else { 1 };
        match specifier {
            b'd' => match count {
                1 | 2 => push_number(out, self.day as i64, count),
                3 => out.push_str(info.get_abbreviated_day_name(value.day_of_week())),
                _ => out.push_str(info.get_day_name(value.day_of_week())),
            },
            b'M' => match count {
                1 | 2 => push_number(out, self.month as i64, count),
                3 => out.push_str(info.get_abbreviated_month_name(self.month)),
                _ => out.push_str(info.get_month_name(self.month)),
            },
            b'y' => match count {
                1 | 2 => push_number(out, (self.year % 100) as i64, count),
                _ => push_number(out, self.year as i64, count),
            },
            b'g' => out.push_str(info.get_era_name(1)),
            b'h' => {
                let hour = value.hour() % 12;
                push_number(out, if hour == 0 { 12 } else { hour } as i64, two(count));
            }
            b'H' => push_number(out, value.hour() as i64, two(count)),
            b'm' => push_number(out, value.minute() as i64, two(count)),
            b's' => push_number(out, value.second() as i64, two(count)),
            b'f' | b'F' => {
                if count > 7 {
                    return Err(bad_format());
                }
                let mut fraction = value.ticks() % TimeSpan::TICKS_PER_SECOND / 10_i64.pow((7 - count) as u32);
                if specifier == b'f' {
                    push_number(out, fraction, count);
                } else {
                    let mut digits = count;
                    while digits > 0 && fraction % 10 == 0 {
                        fraction /= 10;
                        digits -= 1;
                    }
                    if digits > 0 {
                        push_number(out, fraction, digits);
                    } else if out.ends_with('.') {
                        out.pop();
                    }
                }
            }
            b't' => {
                let designator = if value.hour() < 12 { info.am_designator() } else { info.pm_designator() };
                if count == 1 {
                    out.extend(designator.chars().next());
                } else {
                    out.push_str(designator);
                }
            }
            b'z' => push_offset(out, self.zone_offset(), count),
            b'K' => match (self.offset, value.kind()) {
                (Some(offset), _) => push_offset(out, offset, 3),
                (None, DateTimeKind::Unspecified) => {}
                (None, DateTimeKind::Utc) => out.push('Z'),
                (None, DateTimeKind::Local) => push_offset(out, self.zone_offset(), 3),
            },
            b':' => out.push_str(info.time_separator()),
            b'/' => out.push_str(info.date_separator()),
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// The number of times the byte at `index` is repeated from there on.
fn run_length(bytes: &[u8], index: usize) -> usize {
    let byte = bytes[index];
    bytes[index..].iter().take_while(|other| **other == byte).count()
}

/// The specifiers whose repetition forms one token.
fn is_repeatable(byte: u8) -> bool {
    matches!(byte, b'd' | b'M' | b'y' | b'g' | b'h' | b'H' | b'm' | b's' | b'f' | b'F' | b't' | b'z')
}

fn format_custom(
    value: DateTime,
    offset: Option<TimeSpan>,
    pattern: &str,
    info: &DateTimeFormatInfo,
) -> Result<String, FormatError> {
    let (year, month, day, _) = value.date_parts();
    let context = FormatContext { value, offset, info, year, month, day };
    let bytes = pattern.as_bytes();
    let mut out = String::with_capacity(pattern.len() + 8);
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b'\'' | b'"' => {
                let end = find_quote_end(pattern, index)?;
                push_unescaped(&mut out, &pattern[index + 1..end]);
                index = end + 1;
            }
            b'\\' => {
                let escaped = pattern[index + 1..].chars().next().ok_or_else(bad_format)?;
                out.push(escaped);
                index += 1 + escaped.len_utf8();
            }
            b'%' => {
                let next = pattern[index + 1..].chars().next().ok_or_else(bad_format)?;
                if matches!(next, '%' | '\'' | '"' | '\\') {
                    return Err(bad_format());
                }
                if !(next.is_ascii() && context.write(&mut out, next as u8, 1)?) {
                    out.push(next);
                }
                index += 1 + next.len_utf8();
            }
            _ if byte.is_ascii() => {
                let count = if is_repeatable(byte) { run_length(bytes, index) } else { 1 };
                if !context.write(&mut out, byte, count)? {
                    out.push(byte as char);
                }
                index += count;
            }
            _ => {
                let literal = pattern[index..].chars().next().ok_or_else(bad_format)?;
                out.push(literal);
                index += literal.len_utf8();
            }
        }
    }
    Ok(out)
}

/// The index of the quote that closes the quoted literal opened at `start`.
fn find_quote_end(pattern: &str, start: usize) -> Result<usize, FormatError> {
    let bytes = pattern.as_bytes();
    let quote = bytes[start];
    let mut index = start + 1;
    while index < bytes.len() {
        if bytes[index] == quote {
            return Ok(index);
        }
        if bytes[index] == b'\\' {
            index += 1;
            if index >= bytes.len() {
                break;
            }
        }
        index += 1;
    }
    Err(bad_format())
}

/// Appends the content of a quoted literal, resolving backslash escapes.
fn push_unescaped(out: &mut String, literal: &str) {
    let mut escaped = false;
    for c in literal.chars() {
        if c == '\\' && !escaped {
            escaped = true;
        } else {
            out.push(c);
            escaped = false;
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing: shared pieces
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Fields {
    year: Option<i32>,
    month: Option<i32>,
    day: Option<i32>,
    hour: Option<i32>,
    /// The hour was written on a 12-hour clock.
    hour_12: bool,
    minute: Option<i32>,
    second: Option<i32>,
    fraction_ticks: Option<i32>,
    pm: Option<bool>,
    day_of_week: Option<DayOfWeek>,
    offset: Option<TimeSpan>,
    utc_designator: bool,
}

/// Records a field; the same field written twice has to agree.
fn set<T: PartialEq + Copy>(slot: &mut Option<T>, value: T) -> Result<(), FormatError> {
    match slot {
        Some(existing) if *existing != value => Err(bad_format()),
        _ => {
            *slot = Some(value);
            Ok(())
        }
    }
}

fn adjust_two_digit_year(year: i32) -> i32 {
    let century = TWO_DIGIT_YEAR_MAX / 100 * 100;
    if year <= TWO_DIGIT_YEAR_MAX % 100 {
        century + year
    } else {
        century - 100 + year
    }
}

impl Fields {
    fn finish(self, text: &str, styles: DateTimeStyles) -> Result<Parsed, FormatError> {
        let invalid = || not_recognized(text);

        let mut hour = self.hour.unwrap_or(0);
        if self.hour_12 && hour > 12 {
            return Err(invalid());
        }
        // The designator of the afternoon is ignored after an hour of the
        // 24-hour clock ("13:00 PM"); the one of the morning is an error there.
        match self.pm {
            Some(true) if hour < 12 => hour += 12,
            Some(false) if hour == 12 => hour = 0,
            Some(false) if hour > 12 => return Err(invalid()),
            _ => {}
        }

        let (year, month, day) = if self.year.is_none() && self.month.is_none() && self.day.is_none() {
            if styles.contains(DateTimeStyles::NO_CURRENT_DATE_DEFAULT) {
                (1, 1, 1)
            } else {
                let (year, month, day, _) = DateTime::now().date_parts();
                (year, month, day)
            }
        } else {
            let year = match self.year {
                Some(year) => year,
                None => DateTime::now().year(),
            };
            (year, self.month.unwrap_or(1), self.day.unwrap_or(1))
        };

        let date_time = DateTime::try_new_with_time(year, month, day, hour, self.minute.unwrap_or(0), self.second.unwrap_or(0))
            .ok_or_else(invalid)?;
        if self.day_of_week.is_some_and(|day_of_week| day_of_week != date_time.day_of_week()) {
            return Err(invalid());
        }
        let date_time = date_time.try_add_ticks(self.fraction_ticks.unwrap_or(0) as i64).ok_or_else(invalid)?;
        Ok(Parsed { date_time, offset: self.offset, utc_designator: self.utc_designator })
    }
}

/// A position in the text being parsed.
struct Cursor<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Cursor<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.position..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn at_end(&self) -> bool {
        self.position >= self.text.len()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek().filter(|c| c.is_whitespace()) {
            self.position += c.len_utf8();
        }
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.position += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn eat_str(&mut self, expected: &str) -> bool {
        if self.rest().starts_with(expected) {
            self.position += expected.len();
            true
        } else {
            false
        }
    }

    /// Reads between `min` and `max` ASCII digits; gives the value and the
    /// number of digits.
    fn digits(&mut self, min: usize, max: usize) -> Option<(i32, usize)> {
        let rest = self.rest().as_bytes();
        let count = rest.iter().take(max).take_while(|byte| byte.is_ascii_digit()).count();
        if count < min || count == 0 {
            return None;
        }
        let mut value = 0_i32;
        for byte in &rest[..count] {
            value = value.checked_mul(10)?.checked_add((byte - b'0') as i32)?;
        }
        self.position += count;
        Some((value, count))
    }

    /// Reads the digits of a fraction of a second (at most `max`) as ticks.
    fn fraction(&mut self, min: usize, max: usize) -> Option<i32> {
        let rest = self.rest().as_bytes();
        let count = rest.iter().take(max).take_while(|byte| byte.is_ascii_digit()).count();
        if count < min {
            return None;
        }
        let mut ticks = 0_i32;
        for index in 0..7 {
            ticks = ticks * 10 + if index < count { (rest[index] - b'0') as i32 } else { 0 };
        }
        self.position += count;
        Some(ticks)
    }

    /// Matches the longest of `names` at the cursor, ignoring case; gives its index.
    fn name<'n>(&mut self, names: impl IntoIterator<Item = &'n str>) -> Option<usize> {
        let rest = self.rest();
        let best = names
            .into_iter()
            .enumerate()
            .filter_map(|(index, name)| match_ignore_case(rest, name).map(|length| (index, length)))
            .max_by_key(|(index, length)| (*length, std::cmp::Reverse(*index)))?;
        self.position += best.1;
        Some(best.0)
    }

    /// Reads an offset `+h`, `+hh`, `+hhmm` or `+hh:mm` (either sign).
    fn offset(&mut self) -> Option<TimeSpan> {
        let start = self.position;
        let negative = match self.peek() {
            Some('+') => false,
            Some('-') => true,
            _ => return None,
        };
        self.position += 1;
        let result = (|| {
            let (mut hours, digits) = self.digits(1, 4)?;
            let mut minutes = 0;
            if digits > 2 {
                minutes = hours % 100;
                hours /= 100;
            } else if self.eat(':') {
                minutes = self.digits(2, 2)?.0;
            }
            if hours > 14 || minutes > 59 || (hours == 14 && minutes != 0) {
                return None;
            }
            let ticks = (hours as i64 * 60 + minutes as i64) * TimeSpan::TICKS_PER_MINUTE;
            Some(TimeSpan::from_ticks(if negative { -ticks } else { ticks }))
        })();
        if result.is_none() {
            self.position = start;
        }
        result
    }
}

/// The length of the prefix of `text` that equals `name` ignoring case;
/// `None` when `name` is empty or no prefix.
fn match_ignore_case(text: &str, name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    let mut chars = text.char_indices();
    for expected in name.chars() {
        let (_, actual) = chars.next()?;
        if actual != expected && !actual.to_lowercase().eq(expected.to_lowercase()) {
            return None;
        }
    }
    Some(chars.next().map_or(text.len(), |(index, _)| index))
}

fn day_from_index(index: usize) -> DayOfWeek {
    DayOfWeek::ALL[index % 7]
}

// ---------------------------------------------------------------------------
// Exact parsing
// ---------------------------------------------------------------------------

/// The pattern a format string stands for when parsing: a single character
/// is a standard format.
fn expand_for_parsing<'a>(format: &'a str, info: &'a DateTimeFormatInfo) -> Result<Cow<'a, str>, FormatError> {
    let mut chars = format.chars();
    let (first, second) = (chars.next(), chars.next());
    let Some(standard) = first.filter(|_| second.is_none()) else {
        return if format.is_empty() { Err(bad_format()) } else { Ok(Cow::Borrowed(format)) };
    };
    Ok(match standard {
        'd' => Cow::Borrowed(info.short_date_pattern()),
        'D' => Cow::Borrowed(info.long_date_pattern()),
        'f' => Cow::Owned(format!("{} {}", info.long_date_pattern(), info.short_time_pattern())),
        'F' | 'U' => info.full_date_time_pattern(),
        'g' => Cow::Owned(format!("{} {}", info.short_date_pattern(), info.short_time_pattern())),
        'G' => Cow::Owned(format!("{} {}", info.short_date_pattern(), info.long_time_pattern())),
        'm' | 'M' => Cow::Borrowed(info.month_day_pattern()),
        'o' | 'O' => Cow::Borrowed(ROUND_TRIP_PATTERN),
        'r' | 'R' => Cow::Borrowed(info.rfc1123_pattern()),
        's' => Cow::Borrowed(info.sortable_date_time_pattern()),
        't' => Cow::Borrowed(info.short_time_pattern()),
        'T' => Cow::Borrowed(info.long_time_pattern()),
        'u' => Cow::Borrowed(info.universal_sortable_date_time_pattern()),
        'y' | 'Y' => Cow::Borrowed(info.year_month_pattern()),
        _ => return Err(bad_format()),
    })
}

/// Parses `text`, which has to match `format` exactly.
pub(crate) fn parse_exact(
    text: &str,
    format: &str,
    info: &DateTimeFormatInfo,
    styles: DateTimeStyles,
) -> Result<Parsed, FormatError> {
    let invalid = || not_recognized(text);
    let standard = format.chars().count() == 1;
    let invariant = DateTimeFormatInfo::invariant_info();
    // The culture-independent standard formats use the invariant names.
    let info = if standard && matches!(format, "o" | "O" | "r" | "R" | "s" | "u") { &*invariant } else { info };
    let pattern = expand_for_parsing(format, info)?;
    let inner_white = styles.contains(DateTimeStyles::ALLOW_INNER_WHITE);

    let mut cursor = Cursor { text, position: 0 };
    let mut fields = Fields::default();
    if styles.contains(DateTimeStyles::ALLOW_LEADING_WHITE) {
        cursor.skip_whitespace();
    }

    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if inner_white {
            cursor.skip_whitespace();
        }
        match byte {
            b'\'' | b'"' => {
                let end = find_quote_end(&pattern, index)?;
                let mut literal = String::new();
                push_unescaped(&mut literal, &pattern[index + 1..end]);
                if !cursor.eat_str(&literal) {
                    return Err(invalid());
                }
                index = end + 1;
            }
            b'\\' => {
                let escaped = pattern[index + 1..].chars().next().ok_or_else(bad_format)?;
                if !cursor.eat(escaped) {
                    return Err(invalid());
                }
                index += 1 + escaped.len_utf8();
            }
            b'%' => {
                let next = pattern[index + 1..].chars().next().ok_or_else(bad_format)?;
                if matches!(next, '%' | '\'' | '"' | '\\') {
                    return Err(bad_format());
                }
                if !(next.is_ascii() && parse_specifier(&mut cursor, &mut fields, next as u8, 1, info).ok_or_else(invalid)?)
                    && !cursor.eat(next)
                {
                    return Err(invalid());
                }
                index += 1 + next.len_utf8();
            }
            _ if byte.is_ascii() => {
                let count = if is_repeatable(byte) { run_length(bytes, index) } else { 1 };
                if (byte == b'f' || byte == b'F') && count > 7 {
                    return Err(bad_format());
                }
                if !parse_specifier(&mut cursor, &mut fields, byte, count, info).ok_or_else(invalid)? {
                    let literal = byte as char;
                    if literal.is_whitespace() && inner_white {
                        // Any amount of white space was skipped above.
                    } else if !cursor.eat(literal) {
                        return Err(invalid());
                    }
                }
                index += count;
            }
            _ => {
                let literal = pattern[index..].chars().next().ok_or_else(bad_format)?;
                if !cursor.eat(literal) {
                    return Err(invalid());
                }
                index += literal.len_utf8();
            }
        }
    }

    if styles.contains(DateTimeStyles::ALLOW_TRAILING_WHITE) || inner_white {
        cursor.skip_whitespace();
    }
    if !cursor.at_end() {
        return Err(invalid());
    }
    fields.finish(text, styles)
}

/// Reads what `count` repetitions of `specifier` stand for. `Some(false)`
/// when the character is no specifier, `None` when the text does not match.
fn parse_specifier(
    cursor: &mut Cursor<'_>,
    fields: &mut Fields,
    specifier: u8,
    count: usize,
    info: &DateTimeFormatInfo,
) -> Option<bool> {
    // One repetition reads one or two digits, more read exactly that many.
    let number = |cursor: &mut Cursor<'_>, count: usize| {
        if count == 1 {
            cursor.digits(1, 2)
        } else {
            cursor.digits(count.min(2), count.min(2))
        }
    };
    match specifier {
        b'd' => match count {
            1 | 2 => set(&mut fields.day, number(cursor, count)?.0).ok()?,
            3 => {
                let index = cursor.name(info.abbreviated_day_names().iter().map(String::as_str))?;
                set(&mut fields.day_of_week, day_from_index(index)).ok()?;
            }
            _ => {
                let index = cursor.name(info.day_names().iter().map(String::as_str))?;
                set(&mut fields.day_of_week, day_from_index(index)).ok()?;
            }
        },
        b'M' => match count {
            1 | 2 => set(&mut fields.month, number(cursor, count)?.0).ok()?,
            3 => {
                let index = cursor.name(info.abbreviated_month_names().iter().map(String::as_str))?;
                set(&mut fields.month, index as i32 + 1).ok()?;
            }
            _ => {
                let index = cursor.name(info.month_names().iter().map(String::as_str))?;
                set(&mut fields.month, index as i32 + 1).ok()?;
            }
        },
        b'y' => {
            let year = match count {
                1 | 2 => adjust_two_digit_year(number(cursor, count)?.0),
                _ => cursor.digits(count, count)?.0,
            };
            set(&mut fields.year, year).ok()?;
        }
        b'g' => {
            let _ = cursor.name([info.get_era_name(1), "AD"]);
        }
        b'h' => {
            set(&mut fields.hour, number(cursor, count)?.0).ok()?;
            fields.hour_12 = true;
        }
        b'H' => set(&mut fields.hour, number(cursor, count)?.0).ok()?,
        b'm' => set(&mut fields.minute, number(cursor, count)?.0).ok()?,
        b's' => set(&mut fields.second, number(cursor, count)?.0).ok()?,
        b'f' => set(&mut fields.fraction_ticks, cursor.fraction(count, count)?).ok()?,
        b'F' => set(&mut fields.fraction_ticks, cursor.fraction(0, count)?).ok()?,
        b't' => {
            let (am, pm) = (info.am_designator(), info.pm_designator());
            if !(am.is_empty() && pm.is_empty()) {
                let index = if count == 1 {
                    let first = |designator: &'_ str| designator.char_indices().nth(1).map_or(designator.len(), |(index, _)| index);
                    cursor.name([&am[..first(am)], &pm[..first(pm)]])?
                } else {
                    cursor.name([am, pm])?
                };
                set(&mut fields.pm, index == 1).ok()?;
            }
        }
        b'z' => set(&mut fields.offset, cursor.offset()?).ok()?,
        b'K' => {
            if cursor.eat('Z') {
                set(&mut fields.offset, TimeSpan::ZERO).ok()?;
                fields.utc_designator = true;
            } else if let Some(offset) = cursor.offset() {
                set(&mut fields.offset, offset).ok()?;
            }
        }
        b':' => {
            if !cursor.eat_str(info.time_separator()) {
                return None;
            }
        }
        b'/' => {
            if !cursor.eat_str(info.date_separator()) {
                return None;
            }
        }
        _ => return Some(false),
    }
    Some(true)
}

// ---------------------------------------------------------------------------
// Free-form parsing
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum DateOrder {
    YearMonthDay,
    MonthDayYear,
    DayMonthYear,
}

/// The order of the date parts of a culture, read from its short date pattern.
fn date_order(info: &DateTimeFormatInfo) -> DateOrder {
    let pattern = info.short_date_pattern();
    let position = |specifier: char| pattern.find(specifier).unwrap_or(usize::MAX);
    let (year, month, day) = (position('y'), position('M'), position('d'));
    if year < month && year < day {
        DateOrder::YearMonthDay
    } else if day < month {
        DateOrder::DayMonthYear
    } else {
        DateOrder::MonthDayYear
    }
}

/// A number of a date whose meaning is decided once the whole text is read.
#[derive(Clone, Copy)]
struct DateNumber {
    value: i32,
    digits: usize,
}

impl DateNumber {
    fn is_year(self) -> bool {
        self.digits >= 3
    }

    fn year(self) -> i32 {
        if self.digits <= 2 {
            adjust_two_digit_year(self.value)
        } else {
            self.value
        }
    }
}

/// Parses a date and/or time written in the conventions of `info`.
pub(crate) fn parse(text: &str, info: &DateTimeFormatInfo, styles: DateTimeStyles) -> Result<Parsed, FormatError> {
    let invalid = || not_recognized(text);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(invalid());
    }

    let mut cursor = Cursor { text: trimmed, position: 0 };
    let mut fields = Fields::default();
    let mut numbers: Vec<DateNumber> = Vec::with_capacity(3);
    // The count of date numbers read before the month name.
    let mut numbers_before_month_name = 0;
    let mut month_from_name = false;
    let mut time_done = false;
    // Whether the ISO 8601 separator `T` of date and time was read.
    let mut iso_separator = false;

    while let Some(c) = cursor.peek() {
        if c.is_whitespace() {
            cursor.skip_whitespace();
            continue;
        }

        if c.is_ascii_digit() {
            let start = cursor.position;
            let (value, digits) = cursor.digits(1, 9).ok_or_else(invalid)?;
            let time_separator = info.time_separator();
            let is_time = !time_done
                && digits <= 2
                && (cursor.rest().starts_with(':') || (!time_separator.is_empty() && cursor.rest().starts_with(time_separator)));
            if is_time {
                cursor.position = start;
                parse_time(&mut cursor, &mut fields, info).ok_or_else(invalid)?;
                time_done = true;
                continue;
            }
            match cursor.peek() {
                Some('年') => set(&mut fields.year, value).map_err(|_| invalid())?,
                Some('月') => set(&mut fields.month, value).map_err(|_| invalid())?,
                Some('日') => set(&mut fields.day, value).map_err(|_| invalid())?,
                Some('時') if !time_done => set(&mut fields.hour, value).map_err(|_| invalid())?,
                Some('分') if !time_done => set(&mut fields.minute, value).map_err(|_| invalid())?,
                Some('秒') if !time_done => set(&mut fields.second, value).map_err(|_| invalid())?,
                _ => {
                    if numbers.len() == 3 {
                        return Err(invalid());
                    }
                    numbers.push(DateNumber { value, digits });
                    continue;
                }
            }
            cursor.position += 3;
            continue;
        }

        let previous_is_digit = cursor.text[..cursor.position].ends_with(|c: char| c.is_ascii_digit());
        let rest = cursor.rest();
        let next_char = rest.chars().nth(1);

        // The ISO 8601 separator of date and time.
        let iso_date = numbers.len() == 3 && numbers[0].digits == 4;
        if (c == 'T' || c == 't') && iso_date && previous_is_digit && next_char.is_some_and(|next| next.is_ascii_digit()) {
            cursor.position += 1;
            iso_separator = true;
            continue;
        }

        if fields.offset.is_none() {
            if (c == 'Z' || c == 'z') && !next_char.is_some_and(char::is_alphabetic) {
                fields.offset = Some(TimeSpan::ZERO);
                fields.utc_designator = true;
                cursor.position += 1;
                continue;
            }
            // A minus sign is a date separator until the date is complete
            // ("10-12-2024"); after the time or the whole date it starts an
            // offset, as a plus sign does anywhere.
            let date_complete = numbers.len() + usize::from(month_from_name) >= 3;
            if c == '+' || (c == '-' && (time_done || date_complete) && next_char.is_some_and(|next| next.is_ascii_digit())) {
                if let Some(offset) = cursor.offset() {
                    fields.offset = Some(offset);
                    continue;
                }
                if c == '+' {
                    return Err(invalid());
                }
            }
        }

        if matches!(c, '/' | '-' | '.' | ',') || (!info.date_separator().is_empty() && cursor.eat_str(info.date_separator())) {
            if matches!(c, '/' | '-' | '.' | ',') {
                cursor.position += 1;
                // A doubled separator ("10//12") is no date.
                if cursor.peek() == Some(c) {
                    return Err(invalid());
                }
            }
            continue;
        }

        if c.is_alphabetic() {
            // The name of UTC that the culture-independent formats write.
            // It stands apart from the number before it and does not follow
            // the ISO 8601 form with `T`; the name `UTC` is not read.
            if fields.offset.is_none() && !previous_is_digit && !iso_separator {
                if let Some(length) = match_ignore_case(rest, "GMT") {
                    if !rest[length..].starts_with(char::is_alphabetic) {
                        fields.offset = Some(TimeSpan::ZERO);
                        cursor.position += length;
                        continue;
                    }
                }
            }
            parse_name(&mut cursor, &mut fields, info, &mut numbers, &mut time_done, &mut month_from_name, &mut numbers_before_month_name)
                .ok_or_else(invalid)?;
            continue;
        }

        return Err(invalid());
    }

    resolve_date(&mut fields, &numbers, month_from_name, numbers_before_month_name, date_order(info)).ok_or_else(invalid)?;
    if fields.year.is_none() && fields.month.is_none() && fields.day.is_none() && fields.hour.is_none() {
        return Err(invalid());
    }
    fields.finish(text, styles)
}

/// Reads `h:mm[:ss[.fffffff]]` at the cursor.
fn parse_time(cursor: &mut Cursor<'_>, fields: &mut Fields, info: &DateTimeFormatInfo) -> Option<()> {
    let separator = |cursor: &mut Cursor<'_>| {
        cursor.eat(':') || (!info.time_separator().is_empty() && cursor.eat_str(info.time_separator()))
    };
    let hour = cursor.digits(1, 2)?.0;
    if !separator(cursor) {
        return None;
    }
    let minute = cursor.digits(1, 2)?.0;
    let mut second = 0;
    let mut fraction = 0;
    let before_seconds = cursor.position;
    if separator(cursor) {
        match cursor.digits(1, 2) {
            Some((value, _)) => {
                second = value;
                let before_fraction = cursor.position;
                if cursor.eat('.') || cursor.eat(',') {
                    match cursor.fraction(1, 7) {
                        Some(ticks) => {
                            // Digits beyond the resolution round the last tick.
                            let round_up = cursor.peek().is_some_and(|next| ('5'..='9').contains(&next));
                            fraction = ticks + i32::from(round_up);
                            while cursor.peek().is_some_and(|next| next.is_ascii_digit()) {
                                cursor.position += 1;
                            }
                        }
                        None => cursor.position = before_fraction,
                    }
                }
            }
            None => cursor.position = before_seconds,
        }
    }
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    set(&mut fields.hour, hour).ok()?;
    set(&mut fields.minute, minute).ok()?;
    set(&mut fields.second, second).ok()?;
    set(&mut fields.fraction_ticks, fraction).ok()?;
    Some(())
}

/// Reads a month name, a day name or a time designator at the cursor: one
/// of the culture or, failing that, one of the invariant culture (which the
/// culture-independent formats write).
fn parse_name(
    cursor: &mut Cursor<'_>,
    fields: &mut Fields,
    info: &DateTimeFormatInfo,
    numbers: &mut Vec<DateNumber>,
    time_done: &mut bool,
    month_from_name: &mut bool,
    numbers_before_month_name: &mut usize,
) -> Option<()> {
    let start = cursor.position;
    if parse_name_of(cursor, fields, info, numbers, time_done, month_from_name, numbers_before_month_name).is_some() {
        return Some(());
    }
    cursor.position = start;
    let invariant = DateTimeFormatInfo::invariant_info();
    parse_name_of(cursor, fields, &invariant, numbers, time_done, month_from_name, numbers_before_month_name)
}

fn parse_name_of(
    cursor: &mut Cursor<'_>,
    fields: &mut Fields,
    info: &DateTimeFormatInfo,
    numbers: &mut Vec<DateNumber>,
    time_done: &mut bool,
    month_from_name: &mut bool,
    numbers_before_month_name: &mut usize,
) -> Option<()> {
    // Candidates: 0..12 month names, 12..24 abbreviated month names, 24..31
    // day names, 31..38 abbreviated day names, 38 and 39 the designators.
    let abbreviation = |name: &'_ String| name.strip_suffix('.').unwrap_or(name).to_owned();
    let month_abbreviations: Vec<String> = info.abbreviated_month_names()[..12].iter().map(abbreviation).collect();
    let day_abbreviations: Vec<String> = info.abbreviated_day_names().iter().map(abbreviation).collect();
    let candidates = info.month_names()[..12]
        .iter()
        .chain(month_abbreviations.iter())
        .chain(info.day_names().iter())
        .chain(day_abbreviations.iter())
        .map(String::as_str)
        .chain([info.am_designator(), info.pm_designator()]);

    let start = cursor.position;
    let index = cursor.name(candidates)?;
    // A name has to end where the word ends ("Mayo" is not "May").
    let matched = &cursor.text[start..cursor.position];
    if matched.ends_with(|c: char| c.is_ascii_alphabetic()) && cursor.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
        return None;
    }

    match index {
        0..=23 => {
            if *month_from_name {
                return None;
            }
            set(&mut fields.month, (index % 12) as i32 + 1).ok()?;
            *month_from_name = true;
            *numbers_before_month_name = numbers.len();
            cursor.eat('.');
        }
        24..=37 => {
            set(&mut fields.day_of_week, day_from_index(index - 24)).ok()?;
            cursor.eat('.');
        }
        _ => {
            if !*time_done {
                // "5 PM": the number before the designator is the hour.
                let hour = numbers.pop()?;
                if hour.digits > 2 || hour.value > 23 {
                    return None;
                }
                set(&mut fields.hour, hour.value).ok()?;
                *time_done = true;
            }
            set(&mut fields.pm, index == 39).ok()?;
        }
    }
    Some(())
}

/// Decides which of the date numbers are the year, the month and the day.
fn resolve_date(
    fields: &mut Fields,
    numbers: &[DateNumber],
    month_from_name: bool,
    numbers_before_month_name: usize,
    order: DateOrder,
) -> Option<()> {
    if month_from_name {
        match *numbers {
            [] => {
                // A month name alone needs a year or a day stated otherwise.
                if fields.year.is_none() && fields.day.is_none() {
                    return None;
                }
            }
            [single] => {
                if single.is_year() {
                    set(&mut fields.year, single.value).ok()?;
                } else {
                    set(&mut fields.day, single.value).ok()?;
                }
            }
            [first, second] => {
                let year_first = if first.is_year() != second.is_year() {
                    first.is_year()
                } else {
                    // "24 12 Oct" in a culture that writes the year first.
                    numbers_before_month_name == 2 && order == DateOrder::YearMonthDay
                };
                let (year, day) = if year_first { (first, second) } else { (second, first) };
                set(&mut fields.year, year.year()).ok()?;
                set(&mut fields.day, day.value).ok()?;
            }
            _ => return None,
        }
        return Some(());
    }

    match *numbers {
        [] => {}
        // A number alone is no date.
        [_] => return None,
        [first, second] => {
            if first.is_year() {
                set(&mut fields.year, first.value).ok()?;
                set(&mut fields.month, second.value).ok()?;
            } else if second.is_year() {
                set(&mut fields.month, first.value).ok()?;
                set(&mut fields.year, second.value).ok()?;
            } else if order == DateOrder::DayMonthYear {
                set(&mut fields.day, first.value).ok()?;
                set(&mut fields.month, second.value).ok()?;
            } else {
                set(&mut fields.month, first.value).ok()?;
                set(&mut fields.day, second.value).ok()?;
            }
        }
        [first, second, third] => {
            let (year, month, day) = if first.is_year() || order == DateOrder::YearMonthDay {
                (first, second, third)
            } else if order == DateOrder::DayMonthYear {
                (third, second, first)
            } else {
                (third, first, second)
            };
            set(&mut fields.year, year.year()).ok()?;
            set(&mut fields.month, month.value).ok()?;
            set(&mut fields.day, day.value).ok()?;
        }
        _ => return None,
    }
    Some(())
}
