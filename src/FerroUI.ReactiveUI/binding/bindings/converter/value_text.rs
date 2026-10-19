//! The text of booleans, identifiers and time spans as .NET writes and reads it: `Boolean.ToString`
//! and `Boolean.TryParse`, `Guid.ToString("D")` and `Guid.TryParse`, `TimeSpan.ToString()` and
//! `TimeSpan.TryParse` with the invariant culture.

use ferroui_splat::Guid;
use std::time::Duration;

/// `True` or `False` (`Boolean.ToString`).
pub(crate) fn format_bool(value: bool) -> String {
    String::from(if value { "True" } else { "False" })
}

/// `true` or `false` in any case, with white space and null characters around it
/// (`Boolean.TryParse`).
pub(crate) fn parse_bool(text: &str) -> Option<bool> {
    let text = text.trim_matches(|ch: char| ch.is_whitespace() || ch == '\0');
    if text.eq_ignore_ascii_case("True") {
        Some(true)
    } else if text.eq_ignore_ascii_case("False") {
        Some(false)
    } else {
        None
    }
}

/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` in lower case (`Guid.ToString("D")`).
pub(crate) fn format_guid(value: Guid) -> String {
    value.to_string()
}

/// 32 hexadecimal digits, or `None`.
fn parse_hex_32(text: &str) -> Option<u128> {
    if text.len() != 32 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u128::from_str_radix(text, 16).ok()
}

/// The `D` format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
fn parse_guid_d(text: &str) -> Option<u128> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 || [8, 13, 18, 23].iter().any(|&index| bytes[index] != b'-') {
        return None;
    }
    let digits: String = text.chars().filter(|&ch| ch != '-').collect();
    parse_hex_32(&digits)
}

/// One field of the `X` format: `0x` and 1 to `max_digits` hexadecimal digits.
fn parse_guid_x_field(text: &str, max_digits: usize) -> Option<u128> {
    let digits = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))?;
    if digits.is_empty() || digits.len() > max_digits || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u128::from_str_radix(digits, 16).ok()
}

/// The `X` format: `{0xxxxxxxxx,0xxxxx,0xxxxx,{0xxx,0xxx,0xxx,0xxx,0xxx,0xxx,0xxx,0xxx}}`, white
/// space ignored.
fn parse_guid_x(text: &str) -> Option<u128> {
    let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    let inner = compact.strip_prefix('{')?.strip_suffix("}}")?;
    let (head, tail) = inner.split_once(",{")?;
    let head: Vec<&str> = head.split(',').collect();
    let tail: Vec<&str> = tail.split(',').collect();
    if head.len() != 3 || tail.len() != 8 {
        return None;
    }
    let mut value = parse_guid_x_field(head[0], 8)? << 96;
    value |= parse_guid_x_field(head[1], 4)? << 80;
    value |= parse_guid_x_field(head[2], 4)? << 64;
    for (index, field) in tail.iter().enumerate() {
        value |= parse_guid_x_field(field, 2)? << (56 - 8 * index);
    }
    Some(value)
}

/// An identifier in any of the formats of `Guid.TryParse`: `N` (32 digits), `D` (with hyphens),
/// `B` (`D` in braces), `P` (`D` in parentheses) or `X` (the fields in hexadecimal), with white
/// space around it.
pub(crate) fn parse_guid(text: &str) -> Option<Guid> {
    let text = text.trim();
    let bytes = text.as_bytes();
    let value = match bytes.len() {
        32 => parse_hex_32(text),
        36 => parse_guid_d(text),
        38 if (bytes[0] == b'{' && bytes[37] == b'}') || (bytes[0] == b'(' && bytes[37] == b')') => {
            parse_guid_d(&text[1..37])
        }
        _ if bytes.first() == Some(&b'{') => parse_guid_x(text),
        _ => None,
    }?;
    Some(Guid::from_u128(value))
}

const SECONDS_PER_DAY: u64 = 86_400;
/// The nanoseconds of one tick of `TimeSpan`.
const NANOS_PER_TICK: u32 = 100;
/// The largest number of days of a `TimeSpan`.
const MAX_DAYS: u64 = 10_675_199;

/// `[d.]hh:mm:ss[.fffffff]` (`TimeSpan.ToString()`, the constant format `c`).
// Deviation (DEVIATIONS.md, ReactiveUI): `TimeSpan` is `Duration`, which has no sign and counts
// nanoseconds; the text keeps the ticks of 100 nanoseconds that `TimeSpan` has.
pub(crate) fn format_duration(value: Duration) -> String {
    let total_seconds = value.as_secs();
    let days = total_seconds / SECONDS_PER_DAY;
    let hours = (total_seconds / 3_600) % 24;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;
    let ticks = value.subsec_nanos() / NANOS_PER_TICK;
    let mut text = String::with_capacity(26);
    if days > 0 {
        text.push_str(&days.to_string());
        text.push('.');
    }
    text.push_str(&format!("{hours:02}:{minutes:02}:{seconds:02}"));
    if ticks > 0 {
        text.push_str(&format!(".{ticks:07}"));
    }
    text
}

/// 1 to `max_digits` decimal digits, at most `max`.
fn parse_component(text: &str, max_digits: usize, max: u64) -> Option<u64> {
    if text.is_empty() || text.len() > max_digits || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value: u64 = text.parse().ok()?;
    (value <= max).then_some(value)
}

/// A time span as `TimeSpan.TryParse` reads it: `d`, `[d.]hh:mm[:ss[.fffffff]]` or
/// `d:hh:mm:ss[.fffffff]`, with white space around it and an optional `-`.
// Deviation (DEVIATIONS.md, ReactiveUI): a `Duration` cannot be negative, so a negative time span
// other than zero fails.
pub(crate) fn parse_duration(text: &str) -> Option<Duration> {
    let text = text.trim();
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let value = parse_duration_body(body)?;
    if negative && value != Duration::ZERO {
        return None;
    }
    Some(value)
}

fn parse_duration_body(body: &str) -> Option<Duration> {
    let Some(first_colon) = body.find(':') else {
        let days = parse_component(body, 8, MAX_DAYS)?;
        return Some(Duration::from_secs(days * SECONDS_PER_DAY));
    };
    let (days, rest, has_day_prefix) = match body[..first_colon].find('.') {
        Some(dot) => (parse_component(&body[..dot], 8, MAX_DAYS)?, &body[dot + 1..], true),
        None => (0, body, false),
    };
    let parts: Vec<&str> = rest.split(':').collect();
    let (days, hours, minutes, seconds) = match parts.len() {
        2 => (days, parts[0], parts[1], None),
        3 => (days, parts[0], parts[1], Some(parts[2])),
        4 if !has_day_prefix => (parse_component(parts[0], 8, MAX_DAYS)?, parts[1], parts[2], Some(parts[3])),
        _ => return None,
    };
    let hours = parse_component(hours, 2, 23)?;
    let minutes = parse_component(minutes, 2, 59)?;
    let (seconds, nanos) = match seconds {
        None => (0, 0),
        Some(text) => {
            let (seconds, fraction) = match text.split_once('.') {
                Some((seconds, fraction)) => (seconds, Some(fraction)),
                None => (text, None),
            };
            let seconds = parse_component(seconds, 2, 59)?;
            let ticks = match fraction {
                None => 0,
                Some(fraction) => {
                    let value = parse_component(fraction, 7, 9_999_999)?;
                    // The digits are the leading digits of seven.
                    value * 10u64.pow(7 - fraction.len() as u32)
                }
            };
            (seconds, ticks as u32 * NANOS_PER_TICK)
        }
    };
    let total_seconds = days * SECONDS_PER_DAY + hours * 3_600 + minutes * 60 + seconds;
    Some(Duration::new(total_seconds, nanos))
}
