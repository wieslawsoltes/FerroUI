//! Culture-invariant number parsing and formatting helpers.
//!
//! The parsers reproduce the grammar of the reference runtime's invariant-culture
//! number parser (leading/trailing white space, signs, group separators, decimal
//! point, exponent, hex) so that every `parse` in this crate accepts and rejects
//! exactly the same inputs as the reference implementation.

use std::fmt::{self, Write};

/// Flags controlling which syntactic elements a numeric string may contain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct NumberStyles(u32);

impl NumberStyles {
    pub const NONE: Self = Self(0);
    pub const ALLOW_LEADING_WHITE: Self = Self(0x1);
    pub const ALLOW_TRAILING_WHITE: Self = Self(0x2);
    pub const ALLOW_LEADING_SIGN: Self = Self(0x4);
    pub const ALLOW_TRAILING_SIGN: Self = Self(0x8);
    pub const ALLOW_DECIMAL_POINT: Self = Self(0x20);
    pub const ALLOW_THOUSANDS: Self = Self(0x40);
    pub const ALLOW_EXPONENT: Self = Self(0x80);
    pub const ALLOW_HEX_SPECIFIER: Self = Self(0x200);

    /// Leading/trailing white space and a leading sign.
    pub const INTEGER: Self = Self(0x1 | 0x2 | 0x4);
    /// White space, leading or trailing sign, decimal point and group separators.
    pub const NUMBER: Self = Self(0x1 | 0x2 | 0x4 | 0x8 | 0x20 | 0x40);
    /// White space, leading sign, decimal point and exponent.
    pub const FLOAT: Self = Self(0x1 | 0x2 | 0x4 | 0x20 | 0x80);
    /// [`Self::FLOAT`] plus group separators: the default style for parsing a double.
    pub const FLOAT_THOUSANDS: Self = Self(0x1 | 0x2 | 0x4 | 0x20 | 0x80 | 0x40);
    /// White space and hexadecimal digits (no sign, no prefix).
    pub const HEX_NUMBER: Self = Self(0x1 | 0x2 | 0x200);

    #[inline]
    pub const fn bits(self) -> u32 {
        self.0
    }

    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl std::ops::BitOr for NumberStyles {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

/// White space as understood by the number parser: space and U+0009..U+000D only.
#[inline]
const fn is_number_white(c: u8) -> bool {
    c == 0x20 || (c >= 0x09 && c <= 0x0D)
}

struct Scanned {
    negative: bool,
    /// Byte range covering digits, group separators, decimal point and exponent.
    body_start: usize,
    body_end: usize,
    /// Position of the decimal point, if any.
    decimal: Option<usize>,
    /// Position of the exponent marker, if any.
    exponent: Option<usize>,
    has_group: bool,
}

fn scan(b: &[u8], style: NumberStyles) -> Option<Scanned> {
    let n = b.len();
    let mut i = 0;
    let mut sign_seen = false;
    let mut negative = false;

    // Leading white space and sign. White space is not accepted after the sign.
    while i < n {
        let c = b[i];
        if is_number_white(c) && style.contains(NumberStyles::ALLOW_LEADING_WHITE) && !sign_seen {
            i += 1;
        } else if style.contains(NumberStyles::ALLOW_LEADING_SIGN)
            && !sign_seen
            && (c == b'+' || c == b'-')
        {
            sign_seen = true;
            negative = c == b'-';
            i += 1;
        } else {
            break;
        }
    }

    let body_start = i;
    let mut digits = false;
    let mut decimal = None;
    let mut has_group = false;

    while i < n {
        let c = b[i];
        if c.is_ascii_digit() {
            digits = true;
        } else if style.contains(NumberStyles::ALLOW_DECIMAL_POINT)
            && decimal.is_none()
            && c == b'.'
        {
            decimal = Some(i);
        } else if style.contains(NumberStyles::ALLOW_THOUSANDS)
            && digits
            && decimal.is_none()
            && c == b','
        {
            // Group separators are accepted anywhere after the first digit and
            // before the decimal point; group sizes are not validated.
            has_group = true;
        } else {
            break;
        }
        i += 1;
    }

    if !digits {
        return None;
    }

    // An exponent marker that is not followed by digits is left unconsumed
    // (and therefore makes the whole string invalid).
    let mut exponent = None;
    if style.contains(NumberStyles::ALLOW_EXPONENT) && i < n && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < n && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        if j < n && b[j].is_ascii_digit() {
            while j < n && b[j].is_ascii_digit() {
                j += 1;
            }
            exponent = Some(i);
            i = j;
        }
    }

    let body_end = i;

    // Trailing white space and (if no sign was seen yet) a trailing sign.
    while i < n {
        let c = b[i];
        if is_number_white(c) && style.contains(NumberStyles::ALLOW_TRAILING_WHITE) {
            i += 1;
        } else if style.contains(NumberStyles::ALLOW_TRAILING_SIGN)
            && !sign_seen
            && (c == b'+' || c == b'-')
        {
            sign_seen = true;
            negative = c == b'-';
            i += 1;
        } else {
            break;
        }
    }

    if i < n {
        return None;
    }

    Some(Scanned {
        negative,
        body_start,
        body_end,
        decimal,
        exponent,
        has_group,
    })
}

/// Recognizes the invariant spellings of the non-finite values. These are accepted
/// regardless of the requested style, ignoring case and surrounding white space.
fn parse_special_double(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.eq_ignore_ascii_case("Infinity") {
        Some(f64::INFINITY)
    } else if t.eq_ignore_ascii_case("-Infinity") {
        Some(f64::NEG_INFINITY)
    } else if t.eq_ignore_ascii_case("NaN") {
        Some(f64::NAN)
    } else if let Some(rest) = t.strip_prefix('+') {
        if rest.eq_ignore_ascii_case("Infinity") {
            Some(f64::INFINITY)
        } else if rest.eq_ignore_ascii_case("NaN") {
            Some(f64::NAN)
        } else {
            None
        }
    } else if let Some(rest) = t.strip_prefix('-') {
        if rest.eq_ignore_ascii_case("NaN") {
            Some(f64::NAN)
        } else {
            None
        }
    } else {
        None
    }
}

/// Parses a double using the invariant culture and the given style.
pub fn try_parse_double(s: &str, style: NumberStyles) -> Option<f64> {
    let Some(sc) = scan(s.as_bytes(), style) else {
        return parse_special_double(s);
    };

    // The scanned body is pure ASCII, so slicing on its byte offsets is valid.
    let body = &s[sc.body_start..sc.body_end];
    let value: f64 = if sc.has_group {
        let cleaned: String = body.chars().filter(|c| *c != ',').collect();
        cleaned.parse().ok()?
    } else {
        body.parse().ok()?
    };

    Some(if sc.negative { -value } else { value })
}

/// Parses a double with the default style (float syntax plus group separators).
#[inline]
pub fn parse_double(s: &str) -> Option<f64> {
    try_parse_double(s, NumberStyles::FLOAT_THOUSANDS)
}

/// Parses the scanned text as an integer magnitude plus sign.
///
/// A fractional part is accepted only when it consists of zeros. Integer parsing
/// with an exponent is not used anywhere in this crate and is rejected.
fn parse_integer(s: &str, style: NumberStyles) -> Option<i64> {
    let b = s.as_bytes();
    let sc = scan(b, style)?;
    if sc.exponent.is_some() {
        return None;
    }

    let int_end = sc.decimal.unwrap_or(sc.body_end);
    let mut value: i64 = 0;
    for &c in &b[sc.body_start..int_end] {
        if c == b',' {
            continue;
        }
        value = value.checked_mul(10)?.checked_add((c - b'0') as i64)?;
    }

    if let Some(dec) = sc.decimal {
        if b[dec + 1..sc.body_end].iter().any(|&c| c != b'0') {
            return None;
        }
    }

    Some(if sc.negative { -value } else { value })
}

fn parse_hex_u32(s: &str, style: NumberStyles) -> Option<u32> {
    let b = s.as_bytes();
    let n = b.len();
    let mut i = 0;

    if style.contains(NumberStyles::ALLOW_LEADING_WHITE) {
        while i < n && is_number_white(b[i]) {
            i += 1;
        }
    }

    let start = i;
    let mut value: u32 = 0;
    while i < n {
        let d = match b[i] {
            c @ b'0'..=b'9' => c - b'0',
            c @ b'a'..=b'f' => c - b'a' + 10,
            c @ b'A'..=b'F' => c - b'A' + 10,
            _ => break,
        };
        if value > (u32::MAX >> 4) {
            return None; // overflow
        }
        value = (value << 4) | d as u32;
        i += 1;
    }

    if i == start {
        return None;
    }

    if style.contains(NumberStyles::ALLOW_TRAILING_WHITE) {
        while i < n && is_number_white(b[i]) {
            i += 1;
        }
    }

    if i < n {
        return None;
    }

    Some(value)
}

/// Parses an unsigned 32-bit integer using the invariant culture and the given style.
pub fn try_parse_uint(s: &str, style: NumberStyles) -> Option<u32> {
    if style.contains(NumberStyles::ALLOW_HEX_SPECIFIER) {
        parse_hex_u32(s, style)
    } else {
        u32::try_from(parse_integer(s, style)?).ok()
    }
}

/// Parses a signed 32-bit integer using the invariant culture and the given style.
pub fn try_parse_int(s: &str, style: NumberStyles) -> Option<i32> {
    i32::try_from(parse_integer(s, style)?).ok()
}

/// Parses a byte using the invariant culture and the given style.
pub fn try_parse_byte(s: &str, style: NumberStyles) -> Option<u8> {
    u8::try_from(parse_integer(s, style)?).ok()
}

/// Small stack buffer used to post-process `{:e}` output without allocating.
struct StackBuf {
    bytes: [u8; 40],
    len: usize,
}

impl Write for StackBuf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > self.bytes.len() {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// Writes `value` the way the reference runtime prints a double under the invariant
/// culture: the shortest round-trippable digits, plain notation for magnitudes in
/// `[1e-4, 1e15)`, scientific notation (`1E+15`, `1E-05`) otherwise, and
/// `NaN` / `Infinity` / `-Infinity` for the non-finite values.
pub fn write_double<W: Write>(w: &mut W, value: f64) -> fmt::Result {
    if value.is_nan() {
        return w.write_str("NaN");
    }
    if value.is_infinite() {
        return w.write_str(if value > 0.0 { "Infinity" } else { "-Infinity" });
    }

    let abs = value.abs();
    if abs == 0.0 || (1e-4..1e15).contains(&abs) {
        return write!(w, "{}", value);
    }

    let mut buf = StackBuf {
        bytes: [0; 40],
        len: 0,
    };
    write!(buf, "{:e}", value)?;
    let text = std::str::from_utf8(&buf.bytes[..buf.len]).map_err(|_| fmt::Error)?;
    let (mantissa, exponent) = text.split_once('e').ok_or(fmt::Error)?;
    let exponent: i32 = exponent.parse().map_err(|_| fmt::Error)?;
    write!(
        w,
        "{}E{}{:02}",
        mantissa,
        if exponent < 0 { '-' } else { '+' },
        exponent.unsigned_abs()
    )
}

/// `Display` adapter around [`write_double`].
#[derive(Clone, Copy, Debug)]
pub struct InvariantF64(pub f64);

impl fmt::Display for InvariantF64 {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_double(f, self.0)
    }
}

/// `Display` adapter printing a double with exactly two decimals, rounding exact
/// ties away from zero (the reference runtime's fixed-point `F2` format).
#[derive(Clone, Copy, Debug)]
pub struct FixedF2(pub f64);

impl fmt::Display for FixedF2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = self.0;
        if v.is_nan() {
            return f.write_str("NaN");
        }
        if v.is_infinite() {
            return f.write_str(if v > 0.0 { "Infinity" } else { "-Infinity" });
        }

        // A binary double sits exactly on a x.xx5 tie only when it is an odd
        // multiple of 1/8. The standard formatter rounds such ties to even, so
        // nudge them away from zero first.
        let eighths = v * 8.0;
        let is_tie = eighths.fract() == 0.0 && (eighths % 2.0) != 0.0;
        let v = if is_tie { v + 0.001_f64.copysign(v) } else { v };
        write!(f, "{:.2}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(v: f64) -> String {
        InvariantF64(v).to_string()
    }

    #[test]
    fn double_formatting_matches_invariant_rules() {
        assert_eq!(fmt(0.0), "0");
        assert_eq!(fmt(-0.0), "-0");
        assert_eq!(fmt(1.0), "1");
        assert_eq!(fmt(1.5), "1.5");
        assert_eq!(fmt(-12.25), "-12.25");
        assert_eq!(fmt(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(fmt(0.0001), "0.0001");
        assert_eq!(fmt(0.00001), "1E-05");
        assert_eq!(fmt(0.00012345), "0.00012345");
        assert_eq!(fmt(0.00009999), "9.999E-05");
        assert_eq!(fmt(1.5e-7), "1.5E-07");
        assert_eq!(fmt(123456789012345.0), "123456789012345");
        assert_eq!(fmt(1e15), "1E+15");
        assert_eq!(fmt(-1.2345e20), "-1.2345E+20");
        assert_eq!(fmt(1e100), "1E+100");
        assert_eq!(fmt(f64::MAX), "1.7976931348623157E+308");
        assert_eq!(fmt(f64::NAN), "NaN");
        assert_eq!(fmt(f64::INFINITY), "Infinity");
        assert_eq!(fmt(f64::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn fixed_f2_rounds_ties_away_from_zero() {
        assert_eq!(FixedF2(0.5).to_string(), "0.50");
        assert_eq!(FixedF2(1.0).to_string(), "1.00");
        assert_eq!(FixedF2(0.125).to_string(), "0.13");
        assert_eq!(FixedF2(-0.125).to_string(), "-0.13");
        assert_eq!(FixedF2(0.375).to_string(), "0.38");
        assert_eq!(FixedF2(128.0 / 255.0).to_string(), "0.50");
        assert_eq!(FixedF2(0.004).to_string(), "0.00");
    }

    #[test]
    fn float_style_parsing() {
        let f = NumberStyles::FLOAT;
        assert_eq!(try_parse_double("1.5", f), Some(1.5));
        assert_eq!(try_parse_double(" -1.5 ", f), Some(-1.5));
        assert_eq!(try_parse_double("+.5", f), Some(0.5));
        assert_eq!(try_parse_double("5.", f), Some(5.0));
        assert_eq!(try_parse_double("1e3", f), Some(1000.0));
        assert_eq!(try_parse_double("1E-2", f), Some(0.01));
        assert_eq!(try_parse_double("1e", f), None);
        assert_eq!(try_parse_double(".", f), None);
        assert_eq!(try_parse_double("", f), None);
        assert_eq!(try_parse_double("- 5", f), None);
        assert_eq!(try_parse_double("5-", f), None);
        assert_eq!(try_parse_double("1,000", f), None);
        assert_eq!(try_parse_double("inf", f), None);
        assert_eq!(try_parse_double("Infinity", f), Some(f64::INFINITY));
        assert_eq!(try_parse_double("-infinity", f), Some(f64::NEG_INFINITY));
        assert!(try_parse_double("nan", f).unwrap().is_nan());
        assert_eq!(try_parse_double("1e999", f), Some(f64::INFINITY));
        assert_eq!(parse_double("1,000.5"), Some(1000.5));
    }

    #[test]
    fn number_style_parsing() {
        let n = NumberStyles::NUMBER;
        assert_eq!(try_parse_double("5-", n), Some(-5.0));
        assert_eq!(try_parse_double(" 5 - ", n), Some(-5.0));
        assert_eq!(try_parse_double("1,234.5", n), Some(1234.5));
        assert_eq!(try_parse_double("1e3", n), None);
        assert_eq!(try_parse_byte("255", n), Some(255));
        assert_eq!(try_parse_byte(" 12.00 ", n), Some(12));
        assert_eq!(try_parse_byte("12.5", n), None);
        assert_eq!(try_parse_byte("256", n), None);
        assert_eq!(try_parse_byte("-1", n), None);
        assert_eq!(try_parse_byte("-0", n), Some(0));
    }

    #[test]
    fn integer_and_hex_parsing() {
        let i = NumberStyles::INTEGER;
        assert_eq!(try_parse_int("123", i), Some(123));
        assert_eq!(try_parse_int(" -123 ", i), Some(-123));
        assert_eq!(try_parse_int("+7", i), Some(7));
        assert_eq!(try_parse_int("1.0", i), None);
        assert_eq!(try_parse_int("2147483648", i), None);
        assert_eq!(try_parse_int("-2147483648", i), Some(i32::MIN));
        assert_eq!(try_parse_int("abc", i), None);

        let h = NumberStyles::HEX_NUMBER;
        assert_eq!(try_parse_uint("ff8844", h), Some(0xff8844));
        assert_eq!(try_parse_uint("FFFFFFFF", h), Some(0xffff_ffff));
        assert_eq!(try_parse_uint(" ff ", h), Some(0xff));
        assert_eq!(try_parse_uint("1FFFFFFFF", h), None);
        assert_eq!(try_parse_uint("fg", h), None);
        assert_eq!(try_parse_uint("", h), None);
    }
}
