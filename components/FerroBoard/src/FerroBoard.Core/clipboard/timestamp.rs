//! Clipboard-history wire codec over the native Rust calendar type.
//! Only persisted data has the original year/offset/100ns precision constraints.
use jiff::tz::{AmbiguousOffset, TimeZone};
use time::{Date, Month, OffsetDateTime, Time, UtcOffset};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimestampError {
    OutOfRange,
    InvalidFormat,
    UnsupportedLocalTime,
    UnsupportedPrecision,
}
impl std::fmt::Display for TimestampError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::OutOfRange => "Clipboard timestamp cannot be represented by the history format",
            Self::InvalidFormat => "Invalid clipboard timestamp format",
            Self::UnsupportedLocalTime => {
                "Clipboard timestamp requires an available native local time zone"
            }
            Self::UnsupportedPrecision => "Clipboard history timestamps require 100ns precision",
        })
    }
}
impl std::error::Error for TimestampError {}

fn validate_wire_timestamp(value: OffsetDateTime) -> Result<(), TimestampError> {
    let offset = value.offset().whole_seconds();
    let utc = value
        .checked_to_offset(UtcOffset::UTC)
        .ok_or(TimestampError::OutOfRange)?;
    if !(1..=9999).contains(&value.year())
        || !(1..=9999).contains(&utc.year())
        || offset % 60 != 0
        || !(-50_400..=50_400).contains(&offset)
    {
        return Err(TimestampError::OutOfRange);
    }
    if !value.nanosecond().is_multiple_of(100) {
        return Err(TimestampError::UnsupportedPrecision);
    }
    Ok(())
}

/// The default history clock produces values at the persisted format's precision.
/// Injected clocks retain their native values; export rejects unrepresentable data.
pub(crate) fn clipboard_now() -> OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    now.replace_nanosecond(now.nanosecond() / 100 * 100)
        .expect("flooring a nanosecond component stays in range")
}

pub fn format_clipboard_timestamp(value: OffsetDateTime) -> Result<String, TimestampError> {
    validate_wire_timestamp(value)?;
    let mut result = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        value.year(),
        value.month() as u8,
        value.day(),
        value.hour(),
        value.minute(),
        value.second()
    );
    let fraction = value.nanosecond() / 100;
    if fraction != 0 {
        result.push('.');
        result.push_str(format!("{fraction:07}").trim_end_matches('0'));
    }
    let offset = value.offset().whole_minutes();
    result.push(if offset < 0 { '-' } else { '+' });
    result.push_str(&format!(
        "{:02}:{:02}",
        offset.abs() / 60,
        offset.abs() % 60
    ));
    Ok(result)
}
/// Parses a clipboard wire timestamp, using the native system zone only when no
/// offset was persisted. Zone discovery failure is explicit; it never assumes UTC.
pub fn parse_clipboard_timestamp(text: &str) -> Result<OffsetDateTime, TimestampError> {
    parse_timestamp(text, || {
        TimeZone::try_system().map_err(|_| TimestampError::UnsupportedLocalTime)
    })
}

/// Imports a timestamp relative to a caller-owned native zone. Explicit offsets
/// in the wire value take precedence. Useful for deterministic import and tests.
pub fn parse_clipboard_timestamp_in_zone(
    text: &str,
    zone: &TimeZone,
) -> Result<OffsetDateTime, TimestampError> {
    parse_timestamp(text, || {
        if zone.is_unknown() {
            Err(TimestampError::UnsupportedLocalTime)
        } else {
            Ok(zone.clone())
        }
    })
}

fn local_offset(date: Date, time: Time, zone: TimeZone) -> Result<UtcOffset, TimestampError> {
    let local = jiff::civil::DateTime::new(
        date.year() as i16,
        date.month() as i8,
        date.day() as i8,
        time.hour() as i8,
        time.minute() as i8,
        time.second() as i8,
        time.nanosecond() as i32,
    )
    .map_err(|_| TimestampError::OutOfRange)?;
    let offset = match zone.to_ambiguous_timestamp(local).offset() {
        AmbiguousOffset::Unambiguous { offset } => offset,
        AmbiguousOffset::Gap { before, after } | AmbiguousOffset::Fold { before, after } => {
            // The clipboard format keeps the original wall time. Prefer the
            // standard-time side of a DST transition, without normalizing the
            // wall time into a different hour. Other ambiguous transitions use
            // the post-transition rule. The native library owns zone data.
            let a = zone.to_offset_info(
                before
                    .to_timestamp(local)
                    .map_err(|_| TimestampError::OutOfRange)?,
            );
            let b = zone.to_offset_info(
                after
                    .to_timestamp(local)
                    .map_err(|_| TimestampError::OutOfRange)?,
            );
            if a.dst().is_std() != b.dst().is_std() {
                if a.dst().is_std() {
                    a.offset()
                } else {
                    b.offset()
                }
            } else {
                after
            }
        }
    };
    UtcOffset::from_whole_seconds(offset.seconds()).map_err(|_| TimestampError::OutOfRange)
}

fn parse_timestamp(
    text: &str,
    zone: impl FnOnce() -> Result<TimeZone, TimestampError>,
) -> Result<OffsetDateTime, TimestampError> {
    let b = text.as_bytes();
    let bad = TimestampError::InvalidFormat;
    let number = |a: usize, n: usize| -> Result<i64, TimestampError> {
        let p = b.get(a..a + n).ok_or(bad)?;
        if !p.iter().all(u8::is_ascii_digit) {
            return Err(bad);
        }
        Ok(p.iter().fold(0, |v, c| v * 10 + (c - b'0') as i64))
    };
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return Err(bad);
    }
    let y = number(0, 4)?;
    let m = number(5, 2)?;
    let d = number(8, 2)?;
    if !(1..=9999).contains(&y) || !(1..=12).contains(&m) {
        return Err(bad);
    }
    let date = Date::from_calendar_date(
        y as i32,
        Month::try_from(m as u8).map_err(|_| bad)?,
        d as u8,
    )
    .map_err(|_| bad)?;
    if b.len() == 10 {
        let offset = local_offset(date, Time::MIDNIGHT, zone()?)?;
        let value = OffsetDateTime::new_in_offset(date, Time::MIDNIGHT, offset);
        validate_wire_timestamp(value)?;
        return Ok(value);
    }
    if b.get(10) != Some(&b'T') || b.get(13) != Some(&b':') {
        return Err(bad);
    }
    let h = number(11, 2)?;
    let min = number(14, 2)?;
    let mut at = 16;
    let mut sec = 0;
    let mut fraction = 0;
    if b.get(at) == Some(&b':') {
        sec = number(at + 1, 2)?;
        at += 3;
        if b.get(at) == Some(&b'.') {
            at += 1;
            let start = at;
            while b.get(at).is_some_and(u8::is_ascii_digit) {
                if at - start < 7 {
                    fraction = fraction * 10 + (b[at] - b'0') as i64;
                }
                at += 1;
            }
            let n = at - start;
            if n == 0 || n > 16 {
                return Err(bad);
            }
            for _ in n..7 {
                fraction *= 10;
            }
        }
    }
    if h > 23 || min > 59 || sec > 59 {
        return Err(bad);
    }
    let time = Time::from_hms_nano(h as u8, min as u8, sec as u8, (fraction * 100) as u32)
        .map_err(|_| bad)?;
    let offset = match b.get(at) {
        None => local_offset(date, time, zone()?)?,
        Some(b'Z') if at + 1 == b.len() => UtcOffset::UTC,
        Some(sign @ (b'+' | b'-')) => {
            let oh = number(at + 1, 2)?;
            let om = if at + 3 == b.len() {
                0
            } else {
                if b.get(at + 3) != Some(&b':') || at + 6 != b.len() {
                    return Err(bad);
                }
                number(at + 4, 2)?
            };
            if oh > 14 || om > 59 || oh == 14 && om != 0 {
                return Err(bad);
            }
            UtcOffset::from_whole_seconds(
                ((oh * 60 + om) * if *sign == b'-' { -1 } else { 1 }) as i32 * 60,
            )
            .map_err(|_| bad)?
        }
        _ => return Err(bad),
    };
    let value = OffsetDateTime::new_in_offset(date, time, offset);
    validate_wire_timestamp(value)?;
    Ok(value)
}
