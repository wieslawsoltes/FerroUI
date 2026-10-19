//! The text of numbers as .NET writes and reads it with the invariant culture, for the converters
//! of numbers: the standard numeric format strings (`B`, `C`, `D`, `E`, `F`, `G`, `N`, `P`, `R`,
//! `X`), the custom numeric format strings (`0`, `#`, `.`, `,`, `%`, `‰`, `E+0`, sections, quoted
//! literals), and the parsing of `Int32.TryParse` (`NumberStyles.Integer`) and `Double.TryParse`
//! (`NumberStyles.Float | NumberStyles.AllowThousands`).
//!
//! The formatting follows .NET's `Number.Formatting`: a number becomes decimal digits and a scale,
//! which the format then rounds (midpoints away from zero) and lays out. A floating-point number
//! takes its digits from Rust's formatting, which is exact: the shortest digits that read back to
//! the same value for the general format, as .NET Core 3.0 and later write; the digits of the
//! precision asked for otherwise; 15 significant digits (7 for `f32`) for a custom format, as .NET
//! does.
//!
//! The formatter of `ferroui-base` (`utilities::number_format`, for `Decimal`) is the model of this
//! one; both can share one module once phase 1b moves the number types to ferroui-system.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream formats and parses with the current culture
// (en-US in its tests); the port uses the invariant culture, which writes the same text except for
// the currency (`C`: `¤`, negative in parentheses) and percent (`P`: `12.34 %`) formats and the
// symbols of infinity (`Infinity`). A format string that .NET rejects with a `FormatException`
// fails the conversion instead.

use super::conversion_hint::{hint_as_i32, hint_as_str};
use std::any::Any;
use std::fmt::Write;

const NEGATIVE_SIGN: &str = "-";
const POSITIVE_SIGN: &str = "+";
const DECIMAL_SEPARATOR: &str = ".";
const GROUP_SEPARATOR: &str = ",";
const GROUP_SIZE: i32 = 3;
const CURRENCY_SYMBOL: &str = "\u{00A4}";
const PERCENT_SYMBOL: &str = "%";
const PER_MILLE_SYMBOL: &str = "\u{2030}";
const NAN_SYMBOL: &str = "NaN";
const POSITIVE_INFINITY_SYMBOL: &str = "Infinity";
const NEGATIVE_INFINITY_SYMBOL: &str = "-Infinity";
/// `NumberDecimalDigits`, `CurrencyDecimalDigits` and `PercentDecimalDigits` of the invariant
/// culture.
const DEFAULT_DECIMAL_DIGITS: i32 = 2;
/// The default precision of the exponential format (`DefaultPrecisionExponentialFormat`).
const DEFAULT_EXPONENTIAL_DIGITS: i32 = 6;
/// The largest precision of a standard format string (.NET 7 and later).
const MAX_PRECISION: i32 = 999_999_999;

/// Whether the number came from an integer or a floating-point value: an integer that rounds to
/// zero loses its sign, a floating-point value keeps it (`-0`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum NumberKind {
    Integer,
    FloatingPoint,
}

/// A number as decimal digits: `0.d1d2d3.. * 10^scale`, without trailing zeros; zero has no digits
/// (`NumberBuffer`).
struct NumberBuffer {
    digits: Vec<u8>,
    scale: i32,
    negative: bool,
    kind: NumberKind,
}

impl NumberBuffer {
    fn new(mut digits: Vec<u8>, scale: i32, negative: bool, kind: NumberKind) -> Self {
        while digits.last() == Some(&b'0') {
            digits.pop();
        }
        let scale = if digits.is_empty() { 0 } else { scale };
        NumberBuffer { digits, scale, negative, kind }
    }

    fn from_integer(value: i128) -> Self {
        let digits = if value == 0 { Vec::new() } else { value.unsigned_abs().to_string().into_bytes() };
        let scale = digits.len() as i32;
        Self::new(digits, scale, value < 0, NumberKind::Integer)
    }

    /// The digits of Rust's scientific text (`d.ddde-x`) of a non-negative finite value.
    fn from_scientific(text: &str, negative: bool) -> Self {
        let (mantissa, exponent) = text.split_once('e').unwrap_or((text, "0"));
        let exponent: i32 = exponent.parse().unwrap_or(0);
        let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
        if digits.iter().all(|digit| *digit == b'0') {
            digits.clear();
        }
        Self::new(digits, exponent + 1, negative, NumberKind::FloatingPoint)
    }

    /// The digits of Rust's fixed text (`ddd.ddd`) of a non-negative finite value.
    fn from_fixed(text: &str, negative: bool) -> Self {
        let (integral, fraction) = text.split_once('.').unwrap_or((text, ""));
        let mut digits: Vec<u8> = integral.bytes().chain(fraction.bytes()).collect();
        let leading = digits.iter().take_while(|digit| **digit == b'0').count();
        if leading == digits.len() {
            return Self::new(Vec::new(), 0, negative, NumberKind::FloatingPoint);
        }
        digits.drain(..leading);
        Self::new(digits, integral.len() as i32 - leading as i32, negative, NumberKind::FloatingPoint)
    }

    fn count(&self) -> usize {
        self.digits.len()
    }

    /// The digit at `index`, or 0 past the last digit.
    fn digit(&self, index: usize) -> u8 {
        self.digits.get(index).copied().unwrap_or(0)
    }

    /// Rounds to `pos` digits, midpoints away from zero, and removes trailing zeros
    /// (`RoundNumber`).
    fn round(&mut self, pos: i32) {
        let mut i = 0usize;
        while (i as i32) < pos && i < self.count() {
            i += 1;
        }

        if i as i32 == pos && self.digit(i) >= b'5' {
            while i > 0 && self.digits[i - 1] == b'9' {
                i -= 1;
            }
            if i > 0 {
                self.digits[i - 1] += 1;
            } else {
                self.scale += 1;
                if self.digits.is_empty() {
                    self.digits.push(b'1');
                } else {
                    self.digits[0] = b'1';
                }
                i = 1;
            }
        } else {
            while i > 0 && self.digits[i - 1] == b'0' {
                i -= 1;
            }
        }

        if i == 0 {
            if self.kind == NumberKind::Integer {
                // The integer types have no negative zero.
                self.negative = false;
            }
            self.scale = 0;
        }
        self.digits.truncate(i);
    }

    /// Appends the next digit, or `0` when the digits are used up.
    fn push_next(&self, out: &mut String, cursor: &mut usize) {
        let digit = self.digit(*cursor);
        if digit != 0 {
            out.push(digit as char);
            *cursor += 1;
        } else {
            out.push('0');
        }
    }
}

/// A standard format string (a letter and an optional precision, -1 when there is none) or a
/// custom one.
enum FormatSpecifier {
    Standard(char, i32),
    Custom,
}

/// Splits a format string into its specifier and precision (`ParseFormatSpecifier`); `None` when
/// the precision is out of range.
fn parse_format_specifier(format: &str) -> Option<FormatSpecifier> {
    let bytes = format.as_bytes();
    let Some(&first) = bytes.first() else {
        return Some(FormatSpecifier::Standard('G', -1));
    };
    if !first.is_ascii_alphabetic() {
        return Some(FormatSpecifier::Custom);
    }
    if bytes.len() == 1 {
        return Some(FormatSpecifier::Standard(first as char, -1));
    }
    let mut precision: i64 = 0;
    for &byte in &bytes[1..] {
        if !byte.is_ascii_digit() {
            return Some(FormatSpecifier::Custom);
        }
        precision = precision * 10 + i64::from(byte - b'0');
        if precision > i64::from(MAX_PRECISION) {
            return None;
        }
    }
    Some(FormatSpecifier::Standard(first as char, precision as i32))
}

/// A floating-point value of either width.
#[derive(Clone, Copy)]
enum Float {
    Double(f64),
    Single(f32),
}

impl Float {
    fn is_nan(self) -> bool {
        match self {
            Float::Double(value) => value.is_nan(),
            Float::Single(value) => value.is_nan(),
        }
    }

    fn is_infinite(self) -> bool {
        match self {
            Float::Double(value) => value.is_infinite(),
            Float::Single(value) => value.is_infinite(),
        }
    }

    fn is_sign_negative(self) -> bool {
        match self {
            Float::Double(value) => value.is_sign_negative(),
            Float::Single(value) => value.is_sign_negative(),
        }
    }

    /// The significant digits of the general format without a precision: at least 15 for `f64`
    /// and 7 for `f32` (`DoublePrecision`, `SinglePrecision`).
    fn general_digits(self) -> i32 {
        match self {
            Float::Double(_) => 15,
            Float::Single(_) => 7,
        }
    }

    /// The shortest digits that read back to the same value.
    fn shortest(self) -> NumberBuffer {
        let negative = self.is_sign_negative();
        match self {
            Float::Double(value) => NumberBuffer::from_scientific(&format!("{:e}", value.abs()), negative),
            Float::Single(value) => NumberBuffer::from_scientific(&format!("{:e}", value.abs()), negative),
        }
    }

    /// The value rounded to `digits` significant digits (at least one).
    fn significant(self, digits: i32) -> NumberBuffer {
        let negative = self.is_sign_negative();
        let precision = (digits.max(1) - 1) as usize;
        match self {
            Float::Double(value) => NumberBuffer::from_scientific(&format!("{:.*e}", precision, value.abs()), negative),
            Float::Single(value) => NumberBuffer::from_scientific(&format!("{:.*e}", precision, value.abs()), negative),
        }
    }

    /// The value rounded to `decimals` digits after the decimal point.
    fn fixed(self, decimals: i32) -> NumberBuffer {
        let negative = self.is_sign_negative();
        let decimals = decimals.max(0) as usize;
        match self {
            Float::Double(value) => NumberBuffer::from_fixed(&format!("{:.*}", decimals, value.abs()), negative),
            Float::Single(value) => NumberBuffer::from_fixed(&format!("{:.*}", decimals, value.abs()), negative),
        }
    }
}

/// Formats an integer of `bits` bits (two's complement for the hexadecimal and binary formats)
/// with a standard or custom format string (`Int32.ToString(format)` and the like).
pub(crate) fn format_integer(value: i128, bits: u32, format: &str) -> Option<String> {
    match parse_format_specifier(format)? {
        FormatSpecifier::Standard(specifier, precision) => match specifier {
            'G' | 'g' if precision < 1 => Some(value.to_string()),
            'D' | 'd' => {
                let digits = value.unsigned_abs().to_string();
                let mut out = String::with_capacity(digits.len() + 1);
                if value < 0 {
                    out.push_str(NEGATIVE_SIGN);
                }
                let width = precision.max(0) as usize;
                if digits.len() < width {
                    out.extend(std::iter::repeat_n('0', width - digits.len()));
                }
                out.push_str(&digits);
                Some(out)
            }
            'X' | 'x' | 'B' | 'b' => {
                let mask = if bits >= 128 { u128::MAX } else { (1u128 << bits) - 1 };
                let raw = (value as u128) & mask;
                let digits = match specifier {
                    'X' => format!("{raw:X}"),
                    'x' => format!("{raw:x}"),
                    _ => format!("{raw:b}"),
                };
                let width = precision.max(0) as usize;
                let mut out = String::with_capacity(digits.len().max(width));
                if digits.len() < width {
                    out.extend(std::iter::repeat_n('0', width - digits.len()));
                }
                out.push_str(&digits);
                Some(out)
            }
            'R' | 'r' => None,
            _ => number_to_string(&mut NumberBuffer::from_integer(value), specifier, precision),
        },
        FormatSpecifier::Custom => {
            let mut number = NumberBuffer::from_integer(value);
            Some(format_custom(&mut number, format))
        }
    }
}

/// Formats an `f64` (`Double.ToString(format)`); `None` is the general format.
pub(crate) fn format_double(value: f64, format: Option<&str>) -> Option<String> {
    format_float(Float::Double(value), format.unwrap_or(""))
}

/// Formats an `f32` (`Single.ToString(format)`); `None` is the general format.
pub(crate) fn format_single(value: f32, format: Option<&str>) -> Option<String> {
    format_float(Float::Single(value), format.unwrap_or(""))
}

fn format_float(value: Float, format: &str) -> Option<String> {
    let specifier = parse_format_specifier(format)?;
    if value.is_nan() {
        return Some(NAN_SYMBOL.to_string());
    }
    if value.is_infinite() {
        let symbol = if value.is_sign_negative() { NEGATIVE_INFINITY_SYMBOL } else { POSITIVE_INFINITY_SYMBOL };
        return Some(symbol.to_string());
    }
    match specifier {
        FormatSpecifier::Standard(specifier, precision) => match specifier {
            'R' | 'r' | 'G' | 'g' if precision < 1 || specifier.eq_ignore_ascii_case(&'R') => {
                // The shortest round-trippable digits, with at least the precision of the type
                // before the general format switches to scientific notation.
                let mut number = value.shortest();
                let max_digits = (number.count() as i32).max(value.general_digits());
                let general = if specifier.is_ascii_uppercase() { 'G' } else { 'g' };
                number_to_string(&mut number, general, max_digits)
            }
            'G' | 'g' => number_to_string(&mut value.significant(precision), specifier, precision),
            'E' | 'e' => {
                let digits = if precision < 0 { DEFAULT_EXPONENTIAL_DIGITS } else { precision };
                number_to_string(&mut value.significant(digits + 1), specifier, precision)
            }
            'F' | 'f' | 'N' | 'n' | 'C' | 'c' => {
                let decimals = if precision < 0 { DEFAULT_DECIMAL_DIGITS } else { precision };
                number_to_string(&mut value.fixed(decimals), specifier, precision)
            }
            'P' | 'p' => {
                let decimals = if precision < 0 { DEFAULT_DECIMAL_DIGITS } else { precision };
                number_to_string(&mut value.fixed(decimals + 2), specifier, precision)
            }
            _ => None,
        },
        FormatSpecifier::Custom => {
            let mut number = value.significant(value.general_digits());
            Some(format_custom(&mut number, format))
        }
    }
}

/// Lays out `number` with a standard format (`NumberToString`); `None` for a specifier the type
/// does not support.
fn number_to_string(number: &mut NumberBuffer, specifier: char, precision: i32) -> Option<String> {
    let mut out = String::with_capacity(32);
    match specifier {
        'C' | 'c' => {
            let digits = if precision < 0 { DEFAULT_DECIMAL_DIGITS } else { precision };
            number.round(number.scale.saturating_add(digits));
            format_currency(&mut out, number, digits);
        }
        'F' | 'f' => {
            let digits = if precision < 0 { DEFAULT_DECIMAL_DIGITS } else { precision };
            number.round(number.scale.saturating_add(digits));
            if number.negative {
                out.push_str(NEGATIVE_SIGN);
            }
            format_fixed(&mut out, number, digits, false);
        }
        'N' | 'n' => {
            let digits = if precision < 0 { DEFAULT_DECIMAL_DIGITS } else { precision };
            number.round(number.scale.saturating_add(digits));
            if number.negative {
                out.push_str(NEGATIVE_SIGN);
            }
            format_fixed(&mut out, number, digits, true);
        }
        'E' | 'e' => {
            let digits = if precision < 0 { DEFAULT_EXPONENTIAL_DIGITS } else { precision } + 1;
            number.round(digits);
            if number.negative {
                out.push_str(NEGATIVE_SIGN);
            }
            format_scientific(&mut out, number, digits, specifier);
        }
        'G' | 'g' => {
            let digits = if precision < 1 { number.count() as i32 } else { precision };
            number.round(digits);
            if number.negative {
                out.push_str(NEGATIVE_SIGN);
            }
            let exponent = if specifier == 'G' { 'E' } else { 'e' };
            format_general(&mut out, number, digits, exponent);
        }
        'P' | 'p' => {
            let digits = if precision < 0 { DEFAULT_DECIMAL_DIGITS } else { precision };
            number.scale += 2;
            number.round(number.scale.saturating_add(digits));
            // `n %` and `-n %` (`PercentPositivePattern` and `PercentNegativePattern` 0).
            if number.negative {
                out.push_str(NEGATIVE_SIGN);
            }
            format_fixed(&mut out, number, digits, true);
            out.push(' ');
            out.push_str(PERCENT_SYMBOL);
        }
        _ => return None,
    }
    Some(out)
}

/// `¤n` and `(¤n)` (`CurrencyPositivePattern` and `CurrencyNegativePattern` 0).
fn format_currency(out: &mut String, number: &NumberBuffer, digits: i32) {
    if number.negative {
        out.push('(');
    }
    out.push_str(CURRENCY_SYMBOL);
    format_fixed(out, number, digits, true);
    if number.negative {
        out.push(')');
    }
}

/// The integral digits, with group separators when `grouped`, and `max_digits` decimals
/// (`FormatFixed`).
fn format_fixed(out: &mut String, number: &NumberBuffer, mut max_digits: i32, grouped: bool) {
    let mut dig_pos = number.scale;
    let mut cursor = 0usize;

    if dig_pos > 0 {
        if grouped {
            let integral = dig_pos as usize;
            let mut text = String::with_capacity(integral + integral / 3);
            for i in 0..integral {
                if i > 0 && (integral - i) % GROUP_SIZE as usize == 0 {
                    text.push_str(GROUP_SEPARATOR);
                }
                let digit = number.digit(i);
                text.push(if digit != 0 { digit as char } else { '0' });
            }
            out.push_str(&text);
            cursor = integral.min(number.count());
            dig_pos = 0;
        } else {
            while dig_pos > 0 {
                number.push_next(out, &mut cursor);
                dig_pos -= 1;
            }
        }
    } else {
        out.push('0');
    }

    if max_digits > 0 {
        out.push_str(DECIMAL_SEPARATOR);
        if dig_pos < 0 {
            let zeros = (-dig_pos).min(max_digits);
            out.extend(std::iter::repeat_n('0', zeros as usize));
            max_digits -= zeros;
        }
        while max_digits > 0 {
            number.push_next(out, &mut cursor);
            max_digits -= 1;
        }
    }
}

/// `d.dddE+ddd` (`FormatScientific`).
fn format_scientific(out: &mut String, number: &NumberBuffer, mut max_digits: i32, exponent: char) {
    let mut cursor = 0usize;
    number.push_next(out, &mut cursor);
    if max_digits != 1 {
        out.push_str(DECIMAL_SEPARATOR);
    }
    max_digits -= 1;
    while max_digits > 0 {
        number.push_next(out, &mut cursor);
        max_digits -= 1;
    }
    let e = if number.count() == 0 { 0 } else { number.scale - 1 };
    format_exponent(out, e, exponent, 3, true);
}

/// `E`, the sign and at least `min_digits` digits of `value` (`FormatExponent`).
fn format_exponent(out: &mut String, mut value: i32, exponent: char, min_digits: usize, positive_sign: bool) {
    out.push(exponent);
    if value < 0 {
        out.push_str(NEGATIVE_SIGN);
        value = -value;
    } else if positive_sign {
        out.push_str(POSITIVE_SIGN);
    }
    // Writing to a string does not fail.
    let _ = write!(out, "{value:0min_digits$}");
}

/// The digits in fixed notation, or in scientific notation when the decimal exponent is at least
/// `max_digits` or less than -4 (`FormatGeneral`).
fn format_general(out: &mut String, number: &NumberBuffer, max_digits: i32, exponent: char) {
    let mut dig_pos = number.scale;
    let mut scientific = false;
    if dig_pos > max_digits || dig_pos < -3 {
        dig_pos = 1;
        scientific = true;
    }

    let mut cursor = 0usize;
    if dig_pos > 0 {
        while dig_pos > 0 {
            number.push_next(out, &mut cursor);
            dig_pos -= 1;
        }
    } else {
        out.push('0');
    }

    if number.digit(cursor) != 0 || dig_pos < 0 {
        out.push_str(DECIMAL_SEPARATOR);
        while dig_pos < 0 {
            out.push('0');
            dig_pos += 1;
        }
        while number.digit(cursor) != 0 {
            out.push(number.digit(cursor) as char);
            cursor += 1;
        }
    }

    if scientific {
        format_exponent(out, number.scale - 1, exponent, 2, true);
    }
}

/// The character at byte position `pos` and its length in bytes.
fn char_at(text: &str, pos: usize) -> (char, usize) {
    match text[pos..].chars().next() {
        Some(ch) => (ch, ch.len_utf8()),
        None => ('\0', 0),
    }
}

/// The byte position at which a section of a custom format string starts (0: positive,
/// 1: negative, 2: zero); 0 when the format has no such section (`FindSection`).
fn find_section(format: &str, mut section: i32) -> usize {
    if section == 0 {
        return 0;
    }
    let bytes = format.as_bytes();
    let mut src = 0;
    loop {
        if src >= bytes.len() {
            return 0;
        }
        let ch = bytes[src];
        src += 1;
        match ch {
            b'\'' | b'"' => {
                while src < bytes.len() {
                    let quoted = bytes[src];
                    src += 1;
                    if quoted == ch {
                        break;
                    }
                }
            }
            b'\\' => {
                if src < bytes.len() {
                    src += char_at(format, src).1;
                }
            }
            b';' => {
                section -= 1;
                if section != 0 {
                    continue;
                }
                if src < bytes.len() && bytes[src] != b';' {
                    return src;
                }
                return 0;
            }
            _ => {}
        }
    }
}

/// Whether an exponent placeholder (`0`, `+0` or `-0`) follows at `src`.
fn exponent_follows(bytes: &[u8], src: usize) -> bool {
    bytes.get(src) == Some(&b'0') || (matches!(bytes.get(src), Some(b'+' | b'-')) && bytes.get(src + 1) == Some(&b'0'))
}

/// Lays out `number` with a custom format string (`NumberToStringFormat`).
fn format_custom(number: &mut NumberBuffer, format: &str) -> String {
    const UNSET: i32 = i32::MAX;

    let bytes = format.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(format.len() + 16);

    let mut digit_count;
    let mut decimal_pos;
    let mut first_digit;
    let mut last_digit;
    let mut scientific;
    let mut thousand_pos;
    let mut thousand_count = 0;
    let mut thousand_seps;
    let mut scale_adjust;

    let mut section = find_section(format, if number.count() == 0 { 2 } else if number.negative { 1 } else { 0 });

    loop {
        digit_count = 0;
        decimal_pos = -1;
        first_digit = UNSET;
        last_digit = 0;
        scientific = false;
        thousand_pos = -1;
        thousand_seps = false;
        scale_adjust = 0;

        let mut src = section;
        while src < len {
            let (ch, size) = char_at(format, src);
            src += size;
            if ch == ';' {
                break;
            }
            match ch {
                '#' => digit_count += 1,
                '0' => {
                    if first_digit == UNSET {
                        first_digit = digit_count;
                    }
                    digit_count += 1;
                    last_digit = digit_count;
                }
                '.' => {
                    if decimal_pos < 0 {
                        decimal_pos = digit_count;
                    }
                }
                ',' => {
                    if digit_count > 0 && decimal_pos < 0 {
                        if thousand_pos >= 0 {
                            if thousand_pos == digit_count {
                                thousand_count += 1;
                                continue;
                            }
                            thousand_seps = true;
                        }
                        thousand_pos = digit_count;
                        thousand_count = 1;
                    }
                }
                '%' => scale_adjust += 2,
                '\u{2030}' => scale_adjust += 3,
                '\'' | '"' => {
                    while src < len {
                        let quoted = bytes[src];
                        src += 1;
                        if quoted == ch as u8 {
                            break;
                        }
                    }
                }
                '\\' => {
                    if src < len {
                        src += char_at(format, src).1;
                    }
                }
                'E' | 'e' => {
                    if exponent_follows(bytes, src) {
                        src += 1;
                        while src < len && bytes[src] == b'0' {
                            src += 1;
                        }
                        scientific = true;
                    }
                }
                _ => {}
            }
        }

        if decimal_pos < 0 {
            decimal_pos = digit_count;
        }

        if thousand_pos >= 0 {
            if thousand_pos == decimal_pos {
                scale_adjust -= thousand_count * 3;
            } else {
                thousand_seps = true;
            }
        }

        if number.count() != 0 {
            number.scale += scale_adjust;
            let pos = if scientific { digit_count } else { number.scale + digit_count - decimal_pos };
            number.round(pos);
            if number.count() == 0 {
                let zero_section = find_section(format, 2);
                if zero_section != section {
                    section = zero_section;
                    continue;
                }
            }
        } else {
            if number.kind == NumberKind::Integer {
                number.negative = false;
            }
            number.scale = 0;
        }

        break;
    }

    let first_digit = if first_digit < decimal_pos { decimal_pos - first_digit } else { 0 };
    let last_digit = if last_digit > decimal_pos { decimal_pos - last_digit } else { 0 };
    let mut dig_pos;
    let mut adjust;
    if scientific {
        dig_pos = decimal_pos;
        adjust = 0;
    } else {
        dig_pos = number.scale.max(decimal_pos);
        adjust = number.scale - decimal_pos;
    }

    // The positions, counted from the decimal point, after which a group separator goes.
    let mut separator_positions: Vec<i32> = Vec::new();
    if thousand_seps {
        let mut total = GROUP_SIZE;
        let total_digits = dig_pos + adjust.min(0);
        let digits = first_digit.max(total_digits);
        while digits > total {
            separator_positions.push(total);
            total += GROUP_SIZE;
        }
    }

    if number.negative && section == 0 && number.count() > 0 {
        out.push_str(NEGATIVE_SIGN);
    }

    let mut decimal_written = false;
    let mut cursor = 0usize;
    let mut src = section;
    // Appends the group separator when the digit just written ends a group.
    let group = |out: &mut String, dig_pos: i32, positions: &mut Vec<i32>| {
        if thousand_seps && dig_pos > 1 && positions.last().is_some_and(|last| dig_pos == last + 1) {
            out.push_str(GROUP_SEPARATOR);
            positions.pop();
        }
    };

    while src < len {
        let (mut ch, size) = char_at(format, src);
        src += size;
        if ch == ';' {
            break;
        }

        if adjust > 0 && matches!(ch, '#' | '0' | '.') {
            // The number has more integral digits than the format has placeholders: they all go
            // before the first placeholder.
            while adjust > 0 {
                number.push_next(&mut out, &mut cursor);
                group(&mut out, dig_pos, &mut separator_positions);
                dig_pos -= 1;
                adjust -= 1;
            }
        }

        match ch {
            '#' | '0' => {
                if adjust < 0 {
                    adjust += 1;
                    ch = if dig_pos <= first_digit { '0' } else { '\0' };
                } else {
                    let digit = number.digit(cursor);
                    ch = if digit != 0 {
                        cursor += 1;
                        digit as char
                    } else if dig_pos > last_digit {
                        '0'
                    } else {
                        '\0'
                    };
                }
                if ch != '\0' {
                    out.push(ch);
                    group(&mut out, dig_pos, &mut separator_positions);
                }
                dig_pos -= 1;
            }
            '.' => {
                // Repeated decimal points are not echoed.
                if dig_pos != 0 || decimal_written {
                    continue;
                }
                // The format has trailing zeros, or it has decimals and digits remain.
                if last_digit < 0 || (decimal_pos < digit_count && number.digit(cursor) != 0) {
                    out.push_str(DECIMAL_SEPARATOR);
                    decimal_written = true;
                }
            }
            '\u{2030}' => out.push_str(PER_MILLE_SYMBOL),
            '%' => out.push_str(PERCENT_SYMBOL),
            ',' => {}
            '\'' | '"' => {
                let start = src;
                while src < len && bytes[src] != ch as u8 {
                    src += 1;
                }
                out.push_str(&format[start..src]);
                if src < len {
                    src += 1;
                }
            }
            '\\' => {
                if src < len {
                    let (escaped, size) = char_at(format, src);
                    out.push(escaped);
                    src += size;
                }
            }
            'E' | 'e' => {
                if scientific {
                    let mut positive_sign = false;
                    let mut digits = 0usize;
                    if bytes.get(src) == Some(&b'0') {
                        // "E0" formats as "E-0".
                        digits += 1;
                    } else if bytes.get(src) == Some(&b'+') && bytes.get(src + 1) == Some(&b'0') {
                        positive_sign = true;
                    } else if !(bytes.get(src) == Some(&b'-') && bytes.get(src + 1) == Some(&b'0')) {
                        out.push(ch);
                        continue;
                    }

                    src += 1;
                    while src < len && bytes[src] == b'0' {
                        digits += 1;
                        src += 1;
                    }
                    let digits = digits.min(10);

                    let e = if number.count() == 0 { 0 } else { number.scale - decimal_pos };
                    format_exponent(&mut out, e, ch, digits, positive_sign);
                    scientific = false;
                } else {
                    out.push(ch);
                    if src < len {
                        if bytes[src] == b'+' || bytes[src] == b'-' {
                            out.push(bytes[src] as char);
                            src += 1;
                        }
                        while src < len && bytes[src] == b'0' {
                            out.push('0');
                            src += 1;
                        }
                    }
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Formats an integer as the converters of integers do: an `i32` hint is the minimum number of
/// digits (`D{width}`), a string hint is a format string, and no hint is the general format.
pub(crate) fn integer_to_string(value: i128, bits: u32, conversion_hint: Option<&dyn Any>) -> Option<String> {
    if let Some(width) = hint_as_i32(conversion_hint) {
        return format_integer(value, bits, &format!("D{width}"));
    }
    if let Some(format) = hint_as_str(conversion_hint) {
        return format_integer(value, bits, format);
    }
    Some(value.to_string())
}

/// Formats an `f64` as the converters of `double` do: an `i32` hint is the number of decimals
/// (`F{places}`), a string hint is a format string, and no hint is the general format.
pub(crate) fn double_to_string(value: f64, conversion_hint: Option<&dyn Any>) -> Option<String> {
    if let Some(places) = hint_as_i32(conversion_hint) {
        return format_double(value, Some(&format!("F{places}")));
    }
    format_double(value, hint_as_str(conversion_hint))
}

/// Formats an `f32` as the converters of `float` do; the hints are those of
/// [`double_to_string`].
pub(crate) fn single_to_string(value: f32, conversion_hint: Option<&dyn Any>) -> Option<String> {
    if let Some(places) = hint_as_i32(conversion_hint) {
        return format_single(value, Some(&format!("F{places}")));
    }
    format_single(value, hint_as_str(conversion_hint))
}

/// The white space that the parsing of numbers skips: the space and the characters from tab to
/// carriage return.
fn is_white(ch: char) -> bool {
    ch == ' ' || ('\u{09}'..='\u{0D}').contains(&ch)
}

/// `text` without the white space around it and the null characters after it.
fn trim_number(text: &str) -> &str {
    text.trim_end_matches('\0').trim_matches(is_white)
}

/// Parses an integer in `min..=max` (`Int32.TryParse` and the like): white space around it and a
/// leading sign are allowed; `None` when the text is not an integer or is out of range.
pub(crate) fn parse_integer(text: &str, min: i128, max: i128) -> Option<i128> {
    let text = trim_number(text);
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut value: i128 = 0;
    for byte in digits.bytes() {
        value = value.checked_mul(10)?.checked_add(i128::from(byte - b'0'))?;
    }
    let value = if negative { -value } else { value };
    (min..=max).contains(&value).then_some(value)
}

/// What the text of a floating-point number reads as.
enum FloatText {
    /// A number, without its group separators, in the syntax Rust parses.
    Number(String),
    NaN,
    PositiveInfinity,
    NegativeInfinity,
}

/// Reads the text of a floating-point number: white space around it, a leading sign, digits with
/// group separators before the decimal point, decimals and an exponent; or the symbols of NaN and
/// the infinities, in any case.
fn scan_float(text: &str) -> Option<FloatText> {
    let text = trim_number(text);
    let is = |symbol: &str| text.eq_ignore_ascii_case(symbol);
    if is(NAN_SYMBOL) || is("-NaN") || is("+NaN") {
        return Some(FloatText::NaN);
    }
    if is(POSITIVE_INFINITY_SYMBOL) || is("+Infinity") || text == "\u{221E}" || text == "+\u{221E}" {
        return Some(FloatText::PositiveInfinity);
    }
    if is(NEGATIVE_INFINITY_SYMBOL) || text == "-\u{221E}" {
        return Some(FloatText::NegativeInfinity);
    }

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;
    if i < len && (bytes[i] == b'-' || bytes[i] == b'+') {
        out.push(bytes[i] as char);
        i += 1;
    }
    let mut digits = 0;
    while i < len {
        match bytes[i] {
            byte @ b'0'..=b'9' => {
                out.push(byte as char);
                digits += 1;
            }
            b',' if digits > 0 => {}
            _ => break,
        }
        i += 1;
    }
    if i < len && bytes[i] == b'.' {
        out.push('.');
        i += 1;
        while i < len && bytes[i].is_ascii_digit() {
            out.push(bytes[i] as char);
            digits += 1;
            i += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if i < len && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        let mut exponent = String::from("e");
        if j < len && (bytes[j] == b'-' || bytes[j] == b'+') {
            exponent.push(bytes[j] as char);
            j += 1;
        }
        let start = j;
        while j < len && bytes[j].is_ascii_digit() {
            exponent.push(bytes[j] as char);
            j += 1;
        }
        if j > start {
            out.push_str(&exponent);
            i = j;
        }
    }
    if i != len {
        return None;
    }
    Some(FloatText::Number(out))
}

/// Parses an `f64` (`Double.TryParse`); a value too large for the type reads as an infinity, as
/// .NET Core 3.0 and later read it.
pub(crate) fn parse_double(text: &str) -> Option<f64> {
    match scan_float(text)? {
        FloatText::Number(number) => number.parse().ok(),
        FloatText::NaN => Some(f64::NAN),
        FloatText::PositiveInfinity => Some(f64::INFINITY),
        FloatText::NegativeInfinity => Some(f64::NEG_INFINITY),
    }
}

/// Parses an `f32` (`Single.TryParse`).
pub(crate) fn parse_single(text: &str) -> Option<f32> {
    match scan_float(text)? {
        FloatText::Number(number) => number.parse().ok(),
        FloatText::NaN => Some(f32::NAN),
        FloatText::PositiveInfinity => Some(f32::INFINITY),
        FloatText::NegativeInfinity => Some(f32::NEG_INFINITY),
    }
}
