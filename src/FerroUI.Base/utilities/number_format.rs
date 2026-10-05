//! Formatting and parsing of [`Decimal`] numbers with the rules of the
//! managed runtime: the standard numeric format strings (`C`, `E`, `F`,
//! `G`, `N`, `P`, `R`), custom numeric format strings, composite format
//! strings with one decimal argument, and parsing with [`NumberStyles`].
//!
//! Formatting rounds midpoints away from zero, as the managed runtime does
//! when it formats (arithmetic and parsing round them to even).

use super::{Decimal, NumberFormatInfo, NumberStyles};
use crate::data::converters::composite_format::FormatError;
use std::fmt;

/// The error of parsing a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumberParseError {
    /// The text is not a number in the permitted styles (the format
    /// exception of the managed runtime).
    Format,
    /// The number is outside the range of the type (the overflow
    /// exception).
    Overflow,
    /// The styles are not valid for the type (the argument exception).
    InvalidStyle,
}

impl fmt::Display for NumberParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NumberParseError::Format => "Input string was not in a correct format.",
            NumberParseError::Overflow => "Value was either too large or too small for a Decimal.",
            NumberParseError::InvalidStyle => "The number style is not supported on decimal data types.",
        })
    }
}

impl std::error::Error for NumberParseError {}

/// The significant digits a decimal can have.
const DECIMAL_PRECISION: i32 = 29;

/// The digits kept of a number: the precision, one digit to round with and
/// one more, as the managed runtime keeps.
const MAX_DIGITS: usize = 30;

const MAX_MANTISSA: u128 = (1u128 << 96) - 1;

/// A number as decimal digits: `0.d1d2d3.. * 10^scale`.
struct NumberBuffer {
    digits: [u8; MAX_DIGITS + 1],
    count: usize,
    scale: i32,
    negative: bool,
    has_non_zero_tail: bool,
}

impl NumberBuffer {
    fn new() -> Self {
        NumberBuffer { digits: [0; MAX_DIGITS + 1], count: 0, scale: 0, negative: false, has_non_zero_tail: false }
    }

    fn from_decimal(value: Decimal) -> Self {
        let mut reversed = [0u8; MAX_DIGITS];
        let mut count = 0;
        let mut mantissa = value.mantissa();
        while mantissa != 0 {
            reversed[count] = b'0' + (mantissa % 10) as u8;
            mantissa /= 10;
            count += 1;
        }
        let mut number = NumberBuffer::new();
        for (target, source) in number.digits.iter_mut().zip(reversed[..count].iter().rev()) {
            *target = *source;
        }
        number.count = count;
        number.scale = count as i32 - i32::from(value.scale());
        number.negative = value.is_sign_negative();
        number
    }

    /// The digit at `index`, or 0 past the last digit.
    #[inline]
    fn digit(&self, index: usize) -> u8 {
        if index < self.count {
            self.digits[index]
        } else {
            0
        }
    }

    /// Rounds to `pos` digits, midpoints away from zero, and removes
    /// trailing zeros.
    fn round(&mut self, pos: i32) {
        let mut i = 0usize;
        while (i as i32) < pos && i < self.count {
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
                self.digits[0] = b'1';
                i = 1;
            }
        } else {
            while i > 0 && self.digits[i - 1] == b'0' {
                i -= 1;
            }
        }

        if i == 0 {
            // A decimal formats negative zero as zero.
            self.negative = false;
            self.scale = 0;
        }
        self.count = i;
    }

    /// Appends the next digit, or `0` when the digits are used up.
    #[inline]
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

/// Formats a decimal with a standard or custom numeric format string. An
/// empty format string is the general format.
pub fn format_decimal(value: Decimal, format: &str, info: &NumberFormatInfo) -> Result<String, FormatError> {
    let mut number = NumberBuffer::from_decimal(value);
    let mut out = String::with_capacity(32);
    match parse_format_specifier(format)? {
        Some((specifier, digits)) => format_standard(&mut out, &mut number, specifier, digits, info)?,
        None => format_custom(&mut out, &mut number, format, info),
    }
    Ok(out)
}

/// Replaces the format items of a composite format string (`{0}`,
/// `{0,8}`, `{0:N2}`; `{{` and `}}` are braces) with the text of `value`,
/// the only argument. A missing value formats as empty text.
pub fn format_decimal_composite(
    format: &str,
    info: &NumberFormatInfo,
    value: Option<Decimal>,
) -> Result<String, FormatError> {
    fn next(bytes: &[u8], pos: &mut usize) -> Result<u8, FormatError> {
        *pos += 1;
        bytes.get(*pos).copied().ok_or(FormatError::InvalidFormat)
    }

    const LIMIT: usize = 1_000_000;

    let bytes = format.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + 16);
    let mut pos = 0;
    loop {
        // Braces are ASCII: every position the scan stops at is a
        // character boundary.
        let start = pos;
        while pos < len && bytes[pos] != b'{' && bytes[pos] != b'}' {
            pos += 1;
        }
        out.push_str(&format[start..pos]);
        if pos >= len {
            return Ok(out);
        }

        let brace = bytes[pos];
        let mut ch = next(bytes, &mut pos)?;
        if ch == brace {
            out.push(brace as char);
            pos += 1;
            continue;
        }
        if brace != b'{' || !ch.is_ascii_digit() {
            return Err(FormatError::InvalidFormat);
        }

        let mut index = usize::from(ch - b'0');
        ch = next(bytes, &mut pos)?;
        let mut width = 0usize;
        let mut left_justify = false;
        let mut item_format = "";
        if ch != b'}' {
            while ch.is_ascii_digit() && index < LIMIT {
                index = index * 10 + usize::from(ch - b'0');
                ch = next(bytes, &mut pos)?;
            }
            while ch == b' ' {
                ch = next(bytes, &mut pos)?;
            }

            if ch == b',' {
                loop {
                    ch = next(bytes, &mut pos)?;
                    if ch != b' ' {
                        break;
                    }
                }
                if ch == b'-' {
                    left_justify = true;
                    ch = next(bytes, &mut pos)?;
                }
                if !ch.is_ascii_digit() {
                    return Err(FormatError::InvalidFormat);
                }
                width = usize::from(ch - b'0');
                ch = next(bytes, &mut pos)?;
                while ch.is_ascii_digit() && width < LIMIT {
                    width = width * 10 + usize::from(ch - b'0');
                    ch = next(bytes, &mut pos)?;
                }
                while ch == b' ' {
                    ch = next(bytes, &mut pos)?;
                }
            }

            if ch != b'}' {
                if ch != b':' {
                    return Err(FormatError::InvalidFormat);
                }
                let format_start = pos + 1;
                loop {
                    ch = next(bytes, &mut pos)?;
                    if ch == b'}' {
                        break;
                    }
                    if ch == b'{' {
                        return Err(FormatError::InvalidFormat);
                    }
                }
                item_format = &format[format_start..pos];
            }
        }
        pos += 1;

        if index != 0 {
            return Err(FormatError::IndexOutOfRange);
        }
        let text = match value {
            Some(value) => format_decimal(value, item_format, info)?,
            None => String::new(),
        };
        let padding = width.saturating_sub(text.encode_utf16().count());
        if left_justify {
            out.push_str(&text);
            out.extend(std::iter::repeat_n(' ', padding));
        } else {
            out.extend(std::iter::repeat_n(' ', padding));
            out.push_str(&text);
        }
    }
}

/// Splits a standard format string into its specifier and precision (-1
/// when there is none). `None` means the string is a custom format.
fn parse_format_specifier(format: &str) -> Result<Option<(char, i32)>, FormatError> {
    let bytes = format.as_bytes();
    let Some(&first) = bytes.first() else {
        return Ok(Some(('G', -1)));
    };
    if !first.is_ascii_alphabetic() {
        return Ok(if first == 0 { Some(('G', -1)) } else { None });
    }
    if bytes.len() == 1 {
        return Ok(Some((first as char, -1)));
    }

    let mut digits = 0i32;
    for &byte in &bytes[1..] {
        if byte == 0 {
            break;
        }
        if !byte.is_ascii_digit() {
            return Ok(None);
        }
        if digits >= 100_000_000 {
            return Err(FormatError::InvalidFormatSpecifier);
        }
        digits = digits * 10 + i32::from(byte - b'0');
    }
    Ok(Some((first as char, digits)))
}

fn format_standard(
    out: &mut String,
    number: &mut NumberBuffer,
    specifier: char,
    mut digits: i32,
    info: &NumberFormatInfo,
) -> Result<(), FormatError> {
    match specifier.to_ascii_uppercase() {
        'C' => {
            if digits < 0 {
                digits = info.currency_decimal_digits();
            }
            number.round(number.scale.saturating_add(digits));
            format_currency(out, number, digits, info);
        }
        'F' => {
            if digits < 0 {
                digits = info.number_decimal_digits();
            }
            number.round(number.scale.saturating_add(digits));
            if number.negative {
                out.push_str(info.negative_sign());
            }
            format_fixed(out, number, digits, None, info.number_decimal_separator(), "");
        }
        'N' => {
            if digits < 0 {
                digits = info.number_decimal_digits();
            }
            number.round(number.scale.saturating_add(digits));
            format_number(out, number, digits, info);
        }
        'E' => {
            if digits < 0 {
                digits = 6;
            }
            digits += 1;
            number.round(digits);
            if number.negative {
                out.push_str(info.negative_sign());
            }
            format_scientific(out, number, digits, info, specifier);
        }
        'G' | 'R' => {
            let exponent = if specifier.is_ascii_uppercase() { 'E' } else { 'e' };
            let mut no_rounding = false;
            if digits < 1 {
                if digits == -1 {
                    // Without a precision the digits are the digits of the
                    // value, trailing zeros included.
                    no_rounding = true;
                } else {
                    digits = number.count as i32;
                }
            }
            if !no_rounding {
                number.round(digits);
            }
            if number.negative && number.count != 0 {
                out.push_str(info.negative_sign());
            }
            format_general(out, number, digits, info, exponent, no_rounding);
        }
        'P' => {
            if digits < 0 {
                digits = info.percent_decimal_digits();
            }
            number.scale += 2;
            number.round(number.scale.saturating_add(digits));
            format_percent(out, number, digits, info);
        }
        _ => return Err(FormatError::InvalidFormatSpecifier),
    }
    Ok(())
}

pub(crate) const NEGATIVE_NUMBER_FORMATS: [&str; 5] = ["(#)", "-#", "- #", "#-", "# -"];
pub(crate) const POSITIVE_CURRENCY_FORMATS: [&str; 4] = ["$#", "#$", "$ #", "# $"];
pub(crate) const NEGATIVE_CURRENCY_FORMATS: [&str; 17] = [
    "($#)", "-$#", "$-#", "$#-", "(#$)", "-#$", "#-$", "#$-", "-# $", "-$ #", "# $-", "$ #-", "$ -#", "#- $", "($ #)",
    "(# $)", "$- #",
];
pub(crate) const POSITIVE_PERCENT_FORMATS: [&str; 4] = ["# %", "#%", "%#", "% #"];
pub(crate) const NEGATIVE_PERCENT_FORMATS: [&str; 12] =
    ["-# %", "-#%", "-%#", "%-#", "%#-", "#-%", "#%-", "-% #", "# %-", "% #-", "% -#", "#- %"];

/// The pattern at `index`; an index out of range (which the setters do not
/// admit) gives the plain number.
#[inline]
pub(crate) fn pattern(patterns: &[&'static str], index: i32) -> &'static str {
    usize::try_from(index).ok().and_then(|index| patterns.get(index).copied()).unwrap_or("#")
}

fn format_currency(out: &mut String, number: &NumberBuffer, digits: i32, info: &NumberFormatInfo) {
    let format = if number.negative {
        pattern(&NEGATIVE_CURRENCY_FORMATS, info.currency_negative_pattern())
    } else {
        pattern(&POSITIVE_CURRENCY_FORMATS, info.currency_positive_pattern())
    };
    for ch in format.chars() {
        match ch {
            '#' => format_fixed(
                out,
                number,
                digits,
                Some(info.currency_group_sizes()),
                info.currency_decimal_separator(),
                info.currency_group_separator(),
            ),
            '-' => out.push_str(info.negative_sign()),
            '$' => out.push_str(info.currency_symbol()),
            _ => out.push(ch),
        }
    }
}

fn format_number(out: &mut String, number: &NumberBuffer, digits: i32, info: &NumberFormatInfo) {
    let format = if number.negative { pattern(&NEGATIVE_NUMBER_FORMATS, info.number_negative_pattern()) } else { "#" };
    for ch in format.chars() {
        match ch {
            '#' => format_fixed(
                out,
                number,
                digits,
                Some(info.number_group_sizes()),
                info.number_decimal_separator(),
                info.number_group_separator(),
            ),
            '-' => out.push_str(info.negative_sign()),
            _ => out.push(ch),
        }
    }
}

fn format_percent(out: &mut String, number: &NumberBuffer, digits: i32, info: &NumberFormatInfo) {
    let format = if number.negative {
        pattern(&NEGATIVE_PERCENT_FORMATS, info.percent_negative_pattern())
    } else {
        pattern(&POSITIVE_PERCENT_FORMATS, info.percent_positive_pattern())
    };
    for ch in format.chars() {
        match ch {
            '#' => format_fixed(
                out,
                number,
                digits,
                Some(info.percent_group_sizes()),
                info.percent_decimal_separator(),
                info.percent_group_separator(),
            ),
            '-' => out.push_str(info.negative_sign()),
            '%' => out.push_str(info.percent_symbol()),
            _ => out.push(ch),
        }
    }
}

fn format_fixed(
    out: &mut String,
    number: &NumberBuffer,
    mut max_digits: i32,
    group_sizes: Option<&[i32]>,
    decimal_separator: &str,
    group_separator: &str,
) {
    let mut dig_pos = number.scale;
    let mut cursor = 0usize;

    if dig_pos > 0 {
        match group_sizes {
            Some(sizes) => {
                // The integral digits, with the separators, from the right.
                let integral = dig_pos as usize;
                let available = integral.min(number.count);
                let mut size_index = 0;
                let mut size = sizes.first().copied().unwrap_or(0);
                let mut in_group = 0;
                let mut reversed: Vec<&str> = Vec::with_capacity(integral * 2);
                let digits = std::str::from_utf8(&number.digits[..available]).unwrap_or("");
                for i in (0..integral).rev() {
                    reversed.push(if i < available { &digits[i..=i] } else { "0" });
                    if size > 0 {
                        in_group += 1;
                        if in_group == size && i != 0 {
                            reversed.push(group_separator);
                            if size_index + 1 < sizes.len() {
                                size_index += 1;
                                size = sizes[size_index];
                            }
                            in_group = 0;
                        }
                    }
                }
                out.extend(reversed.into_iter().rev());
                cursor = available;
                dig_pos = 0;
            }
            None => {
                while dig_pos > 0 {
                    number.push_next(out, &mut cursor);
                    dig_pos -= 1;
                }
            }
        }
    } else {
        out.push('0');
    }

    if max_digits > 0 {
        out.push_str(decimal_separator);
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

fn format_scientific(out: &mut String, number: &NumberBuffer, mut max_digits: i32, info: &NumberFormatInfo, exponent: char) {
    let mut cursor = 0usize;
    number.push_next(out, &mut cursor);
    if max_digits != 1 {
        out.push_str(info.number_decimal_separator());
    }
    max_digits -= 1;
    while max_digits > 0 {
        number.push_next(out, &mut cursor);
        max_digits -= 1;
    }

    let e = if number.count == 0 { 0 } else { number.scale - 1 };
    format_exponent(out, info, e, exponent, 3, true);
}

fn format_exponent(
    out: &mut String,
    info: &NumberFormatInfo,
    mut value: i32,
    exponent: char,
    min_digits: usize,
    positive_sign: bool,
) {
    use std::fmt::Write;

    out.push(exponent);
    if value < 0 {
        out.push_str(info.negative_sign());
        value = -value;
    } else if positive_sign {
        out.push_str(info.positive_sign());
    }
    // Writing to a string does not fail.
    let _ = write!(out, "{value:0min_digits$}");
}

fn format_general(
    out: &mut String,
    number: &NumberBuffer,
    max_digits: i32,
    info: &NumberFormatInfo,
    exponent: char,
    suppress_scientific: bool,
) {
    let mut dig_pos = number.scale;
    let mut scientific = false;
    if !suppress_scientific && (dig_pos > max_digits || dig_pos < -3) {
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
        out.push_str(info.number_decimal_separator());
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
        format_exponent(out, info, number.scale - 1, exponent, 2, true);
    }
}

/// The character at byte position `pos` and its length in bytes.
#[inline]
fn char_at(text: &str, pos: usize) -> (char, usize) {
    match text[pos..].chars().next() {
        Some(ch) => (ch, ch.len_utf8()),
        None => ('\0', 0),
    }
}

/// The byte position at which a section of a custom format string starts
/// (0: positive, 1: negative, 2: zero); 0 when the format has no such
/// section.
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
                while src < bytes.len() && bytes[src] != 0 {
                    let quoted = bytes[src];
                    src += 1;
                    if quoted == ch {
                        break;
                    }
                }
            }
            b'\\' => {
                if src < bytes.len() && bytes[src] != 0 {
                    src += char_at(format, src).1;
                }
            }
            b';' => {
                section -= 1;
                if section != 0 {
                    continue;
                }
                if src < bytes.len() && bytes[src] != 0 && bytes[src] != b';' {
                    return src;
                }
                return 0;
            }
            0 => return 0,
            _ => {}
        }
    }
}

/// Whether an exponent placeholder (`0`, `+0` or `-0`) follows at `src`.
#[inline]
fn exponent_follows(bytes: &[u8], src: usize) -> bool {
    bytes.get(src) == Some(&b'0')
        || (matches!(bytes.get(src), Some(b'+' | b'-')) && bytes.get(src + 1) == Some(&b'0'))
}

fn format_custom(out: &mut String, number: &mut NumberBuffer, format: &str, info: &NumberFormatInfo) {
    const UNSET: i32 = i32::MAX;

    let bytes = format.as_bytes();
    let len = bytes.len();

    let mut digit_count;
    let mut decimal_pos;
    let mut first_digit;
    let mut last_digit;
    let mut scientific;
    let mut thousand_pos;
    let mut thousand_count = 0;
    let mut thousand_seps;
    let mut scale_adjust;

    let mut section = find_section(format, if number.count == 0 { 2 } else if number.negative { 1 } else { 0 });

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
            if ch == '\0' || ch == ';' {
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
                    while src < len && bytes[src] != 0 {
                        let quoted = bytes[src];
                        src += 1;
                        if quoted == ch as u8 {
                            break;
                        }
                    }
                }
                '\\' => {
                    if src < len && bytes[src] != 0 {
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

        if number.count != 0 {
            number.scale += scale_adjust;
            let pos = if scientific { digit_count } else { number.scale + digit_count - decimal_pos };
            number.round(pos);
            if number.count == 0 {
                let zero_section = find_section(format, 2);
                if zero_section != section {
                    section = zero_section;
                    continue;
                }
            }
        } else {
            number.negative = false;
            // Decimals with a scale ("0.00") are rounded.
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

    // The positions, counted from the decimal point, after which a group
    // separator goes: the format is walked forwards.
    let mut separator_positions: Vec<i32> = Vec::new();
    if thousand_seps && !info.number_group_separator().is_empty() {
        let sizes = info.number_group_sizes();
        let mut size_index = 0;
        let mut total = sizes.first().copied().unwrap_or(0);
        let mut size = total;

        let total_digits = dig_pos + adjust.min(0);
        let digits = first_digit.max(total_digits);
        while digits > total {
            if size == 0 {
                break;
            }
            separator_positions.push(total);
            if size_index + 1 < sizes.len() {
                size_index += 1;
                size = sizes[size_index];
            }
            total += size;
        }
    }

    if number.negative && section == 0 && number.count > 0 {
        out.push_str(info.negative_sign());
    }

    let mut decimal_written = false;
    let mut cursor = 0usize;
    let mut src = section;
    // Appends the group separator when the digit just written ends a group.
    let group = |out: &mut String, dig_pos: i32, positions: &mut Vec<i32>| {
        if thousand_seps && dig_pos > 1 && positions.last().is_some_and(|last| dig_pos == last + 1) {
            out.push_str(info.number_group_separator());
            positions.pop();
        }
    };

    while src < len {
        let (mut ch, size) = char_at(format, src);
        src += size;
        if ch == '\0' || ch == ';' {
            break;
        }

        if adjust > 0 && matches!(ch, '#' | '0' | '.') {
            // The number has more integral digits than the format has
            // placeholders: they all go before the first placeholder.
            while adjust > 0 {
                number.push_next(out, &mut cursor);
                group(out, dig_pos, &mut separator_positions);
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
                    group(out, dig_pos, &mut separator_positions);
                }
                dig_pos -= 1;
            }
            '.' => {
                // Repeated decimal points are not echoed.
                if dig_pos != 0 || decimal_written {
                    continue;
                }
                // The format has trailing zeros, or it has decimals and
                // digits remain.
                if last_digit < 0 || (decimal_pos < digit_count && number.digit(cursor) != 0) {
                    out.push_str(info.number_decimal_separator());
                    decimal_written = true;
                }
            }
            '\u{2030}' => out.push_str(info.per_mille_symbol()),
            '%' => out.push_str(info.percent_symbol()),
            ',' => {}
            '\'' | '"' => {
                let start = src;
                while src < len && bytes[src] != 0 && bytes[src] != ch as u8 {
                    src += 1;
                }
                out.push_str(&format[start..src]);
                if src < len && bytes[src] != 0 {
                    src += 1;
                }
            }
            '\\' => {
                if src < len && bytes[src] != 0 {
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

                    let e = if number.count == 0 { 0 } else { number.scale - decimal_pos };
                    format_exponent(out, info, e, ch, digits, positive_sign);
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
}

const STATE_SIGN: u32 = 0x0001;
const STATE_PARENS: u32 = 0x0002;
const STATE_DIGITS: u32 = 0x0004;
const STATE_NON_ZERO: u32 = 0x0008;
const STATE_DECIMAL: u32 = 0x0010;
const STATE_CURRENCY: u32 = 0x0020;

#[inline]
fn is_white(ch: char) -> bool {
    ch == ' ' || ('\u{09}'..='\u{0D}').contains(&ch)
}

/// The position after `pattern` when the text continues with it at `pos`.
/// A no-break space of the pattern also matches a plain space.
fn match_chars(text: &str, pos: usize, pattern: &str) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }
    let mut rest = text[pos..].chars();
    let mut end = pos;
    for expected in pattern.chars() {
        let actual = rest.next()?;
        if actual != expected && !(matches!(expected, '\u{00A0}' | '\u{202F}') && actual == ' ') {
            return None;
        }
        end += actual.len_utf8();
    }
    Some(end)
}

/// Matches the negative sign; a hyphen also matches when the sign is one of
/// the other minus characters.
fn match_negative_sign(text: &str, pos: usize, info: &NumberFormatInfo) -> Option<usize> {
    if let Some(next) = match_chars(text, pos, info.negative_sign()) {
        return Some(next);
    }
    let mut sign = info.negative_sign().chars();
    let allow_hyphen = matches!(
        (sign.next(), sign.next()),
        (Some('\u{2012}' | '\u{207B}' | '\u{208B}' | '\u{2212}' | '\u{2796}' | '\u{FE63}' | '\u{FF0D}'), None)
    );
    if allow_hyphen && text.as_bytes().get(pos) == Some(&b'-') {
        Some(pos + 1)
    } else {
        None
    }
}

/// Parses a decimal with the given styles and format information.
pub fn parse_decimal(text: &str, styles: NumberStyles, info: &NumberFormatInfo) -> Result<Decimal, NumberParseError> {
    let undefined = styles.bits() & !NumberStyles::all().bits() != 0;
    if undefined || styles.intersects(NumberStyles::ALLOW_HEX_SPECIFIER | NumberStyles::ALLOW_BINARY_SPECIFIER) {
        return Err(NumberParseError::InvalidStyle);
    }

    let number = parse_number(text, styles, info).ok_or(NumberParseError::Format)?;
    number_to_decimal(&number).ok_or(NumberParseError::Overflow)
}

fn parse_number(text: &str, styles: NumberStyles, info: &NumberFormatInfo) -> Option<NumberBuffer> {
    let mut number = NumberBuffer::new();

    let (decimal_separator, group_separator, parsing_currency) =
        if styles.contains(NumberStyles::ALLOW_CURRENCY_SYMBOL) {
            // The currency separators are matched first and, failing that,
            // the number separators.
            (info.currency_decimal_separator(), info.currency_group_separator(), true)
        } else {
            (info.number_decimal_separator(), info.number_group_separator(), false)
        };
    let mut currency_symbol = parsing_currency.then(|| info.currency_symbol());

    let mut state = 0u32;
    let mut pos = 0usize;

    loop {
        let (ch, size) = char_at(text, pos);
        // White space is skipped unless a sign was found that is not
        // followed by a currency symbol: "-Kr 1231.47" is legal but
        // "- 1231.47" is not.
        if !is_white(ch)
            || !styles.contains(NumberStyles::ALLOW_LEADING_WHITE)
            || (state & STATE_SIGN != 0 && state & STATE_CURRENCY == 0 && info.number_negative_pattern() != 2)
        {
            if styles.contains(NumberStyles::ALLOW_LEADING_SIGN) && state & STATE_SIGN == 0 {
                if let Some(next) = match_chars(text, pos, info.positive_sign()) {
                    state |= STATE_SIGN;
                    pos = next;
                    continue;
                }
                if let Some(next) = match_negative_sign(text, pos, info) {
                    number.negative = true;
                    state |= STATE_SIGN;
                    pos = next;
                    continue;
                }
            }
            if ch == '(' && styles.contains(NumberStyles::ALLOW_PARENTHESES) && state & STATE_SIGN == 0 {
                state |= STATE_SIGN | STATE_PARENS;
                number.negative = true;
            } else if let Some(next) = currency_symbol.and_then(|symbol| match_chars(text, pos, symbol)) {
                state |= STATE_CURRENCY;
                // There is at most one currency symbol.
                currency_symbol = None;
                pos = next;
                continue;
            } else {
                break;
            }
        }
        pos += size;
    }

    let mut digit_count = 0usize;
    let mut digit_end = 0usize;
    loop {
        let (ch, size) = char_at(text, pos);
        if ch.is_ascii_digit() {
            state |= STATE_DIGITS;

            if ch != '0' || state & STATE_NON_ZERO != 0 {
                if digit_count < MAX_DIGITS {
                    number.digits[digit_count] = ch as u8;
                    digit_end = digit_count + 1;
                } else if ch != '0' {
                    // Digits past the precision only matter for rounding.
                    number.has_non_zero_tail = true;
                }

                if state & STATE_DECIMAL == 0 {
                    number.scale += 1;
                }
                digit_count += 1;
                state |= STATE_NON_ZERO;
            } else if state & STATE_DECIMAL != 0 {
                number.scale -= 1;
            }
            pos += size;
        } else if styles.contains(NumberStyles::ALLOW_DECIMAL_POINT) && state & STATE_DECIMAL == 0 {
            let next = match_chars(text, pos, decimal_separator).or_else(|| {
                (parsing_currency && state & STATE_CURRENCY == 0)
                    .then(|| match_chars(text, pos, info.number_decimal_separator()))
                    .flatten()
            });
            match next {
                Some(next) => {
                    state |= STATE_DECIMAL;
                    pos = next;
                }
                None => match match_group(text, pos, styles, state, group_separator, parsing_currency, info) {
                    Some(next) => pos = next,
                    None => break,
                },
            }
        } else {
            match match_group(text, pos, styles, state, group_separator, parsing_currency, info) {
                Some(next) => pos = next,
                None => break,
            }
        }
    }

    number.count = digit_end;

    if state & STATE_DIGITS == 0 {
        return None;
    }

    let (ch, _) = char_at(text, pos);
    if (ch == 'E' || ch == 'e') && styles.contains(NumberStyles::ALLOW_EXPONENT) {
        let mut next = pos + 1;
        let mut negative_exponent = false;
        if let Some(after) = match_chars(text, next, info.positive_sign()) {
            next = after;
        } else if let Some(after) = match_negative_sign(text, next, info) {
            next = after;
            negative_exponent = true;
        }

        if char_at(text, next).0.is_ascii_digit() {
            let mut exponent = 0i32;
            loop {
                let (ch, _) = char_at(text, next);
                if !ch.is_ascii_digit() {
                    break;
                }
                next += 1;
                if exponent <= 1000 {
                    exponent = exponent * 10 + (ch as i32 - '0' as i32);
                    if exponent > 1000 {
                        exponent = 9999;
                    }
                }
            }
            if negative_exponent {
                exponent = -exponent;
            }
            number.scale += exponent;
            pos = next;
        }
    }

    loop {
        let (ch, size) = char_at(text, pos);
        if !is_white(ch) || !styles.contains(NumberStyles::ALLOW_TRAILING_WHITE) {
            if styles.contains(NumberStyles::ALLOW_TRAILING_SIGN) && state & STATE_SIGN == 0 {
                if let Some(next) = match_chars(text, pos, info.positive_sign()) {
                    state |= STATE_SIGN;
                    pos = next;
                    continue;
                }
                if let Some(next) = match_negative_sign(text, pos, info) {
                    number.negative = true;
                    state |= STATE_SIGN;
                    pos = next;
                    continue;
                }
            }
            if ch == ')' && state & STATE_PARENS != 0 {
                state &= !STATE_PARENS;
            } else if let Some(next) = currency_symbol.and_then(|symbol| match_chars(text, pos, symbol)) {
                currency_symbol = None;
                pos = next;
                continue;
            } else {
                break;
            }
        }
        pos += size;
    }

    if state & STATE_PARENS != 0 {
        return None;
    }

    // Only null characters may follow the number.
    if text[pos..].bytes().any(|byte| byte != 0) {
        return None;
    }

    Some(number)
}

/// Matches a group separator, which is permitted between integral digits.
fn match_group(
    text: &str,
    pos: usize,
    styles: NumberStyles,
    state: u32,
    group_separator: &str,
    parsing_currency: bool,
    info: &NumberFormatInfo,
) -> Option<usize> {
    if !styles.contains(NumberStyles::ALLOW_THOUSANDS) || state & STATE_DIGITS == 0 || state & STATE_DECIMAL != 0 {
        return None;
    }
    match_chars(text, pos, group_separator).or_else(|| {
        (parsing_currency && state & STATE_CURRENCY == 0)
            .then(|| match_chars(text, pos, info.number_group_separator()))
            .flatten()
    })
}

/// The decimal of parsed digits; digits past the precision are rounded
/// half to even. `None` when the number is out of range.
fn number_to_decimal(number: &NumberBuffer) -> Option<Decimal> {
    const LIMIT: u128 = MAX_MANTISSA / 10;

    let mut index = 0usize;
    let mut c = number.digit(0);
    let mut e = number.scale;
    if c == 0 {
        return Some(Decimal::from_parts(0, (-e).clamp(0, 28) as u8, number.negative));
    }
    if e > DECIMAL_PRECISION {
        return None;
    }

    let mut mantissa = 0u128;
    while (e > 0 || (c != 0 && e > -28)) && (mantissa < LIMIT || (mantissa == LIMIT && c <= b'5')) {
        mantissa *= 10;
        if c != 0 {
            mantissa += u128::from(c - b'0');
            index += 1;
            c = number.digit(index);
        }
        e -= 1;
    }

    if c >= b'5' {
        let mut round_up = true;
        if c == b'5' && mantissa & 1 == 0 {
            index += 1;
            c = number.digit(index);
            let mut zero_tail = !number.has_non_zero_tail;
            while c != 0 && zero_tail {
                zero_tail &= c == b'0';
                index += 1;
                c = number.digit(index);
            }
            // An exact midpoint stays at the even digit.
            round_up = !zero_tail;
        }

        if round_up {
            mantissa += 1;
            if mantissa > MAX_MANTISSA {
                mantissa = LIMIT + 1;
                e += 1;
            }
        }
    }

    if e > 0 {
        return None;
    }
    if e <= -DECIMAL_PRECISION {
        // A scale of more digits than the precision: the number rounds to
        // zero.
        return Some(Decimal::from_parts(0, 28, number.negative));
    }
    Some(Decimal::from_parts(mantissa, (-e) as u8, number.negative))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::{CultureInfo, TestCultureDataProvider};
    use crate::FerroLocator;
    use std::rc::Rc;

    fn d(text: &str) -> Decimal {
        Decimal::parse(text).unwrap()
    }

    fn f(value: &str, format: &str) -> String {
        d(value).to_string_with(format, Some(&NumberFormatInfo::invariant_info())).unwrap()
    }

    /// The conventions of `en-US`, from the provider of the tests.
    fn english() -> Rc<NumberFormatInfo> {
        let _scope = FerroLocator::enter_scope();
        TestCultureDataProvider::register();
        CultureInfo::get_culture_info("en-US").number_format()
    }

    fn us(value: &str, format: &str) -> String {
        d(value).to_string_with(format, Some(&english())).unwrap()
    }

    #[test]
    fn general_format() {
        assert_eq!(f("1.10", ""), "1.10");
        assert_eq!(f("1.10", "G"), "1.10");
        assert_eq!(f("-1.10", "g"), "-1.10");
        assert_eq!(f("0.00", "G"), "0.00");
        assert_eq!(f("-0.00", "G"), "0.00");
        assert_eq!(f("0", "G"), "0");
        assert_eq!(f("12345.6789", "G"), "12345.6789");
        assert_eq!(f("0.0000001", "G"), "0.0000001");
        assert_eq!(f("79228162514264337593543950335", "G"), "79228162514264337593543950335");
        // With a precision the value is rounded and trailing zeros go.
        assert_eq!(f("1.10", "G0"), "1.1");
        assert_eq!(f("12345.6789", "G4"), "1.235E+04");
        assert_eq!(f("12345.6789", "g4"), "1.235e+04");
        assert_eq!(f("12345.6789", "G7"), "12345.68");
        assert_eq!(f("0.00001234", "G2"), "1.2E-05");
        assert_eq!(f("0.0001234", "G2"), "0.00012");
        assert_eq!(f("100", "G0"), "100");
        assert_eq!(f("2.5", "G1"), "3");
        assert_eq!(f("1.10", "R"), "1.10");
        assert_eq!(d("1.50").to_string_provider(&NumberFormatInfo::new().with_number_decimal_separator(";")), "1;50");
    }

    #[test]
    fn fixed_point_format() {
        assert_eq!(f("10.11", "F0"), "10");
        assert_eq!(f("10.11", "F2"), "10.11");
        assert_eq!(f("10.11", "F4"), "10.1100");
        assert_eq!(f("10.11", "F"), "10.11");
        assert_eq!(f("10.115", "F2"), "10.12");
        assert_eq!(f("10.125", "F2"), "10.13");
        assert_eq!(f("-10.125", "f2"), "-10.13");
        assert_eq!(f("0.5", "F0"), "1");
        assert_eq!(f("2.5", "F0"), "3");
        assert_eq!(f("-0.4", "F0"), "0");
        assert_eq!(f("0.004", "F2"), "0.00");
        assert_eq!(f("0.005", "F2"), "0.01");
        assert_eq!(f("999.999", "F2"), "1000.00");
        assert_eq!(f("1234567", "F1"), "1234567.0");
        assert_eq!(f("0.000", "F1"), "0.0");
    }

    #[test]
    fn number_format() {
        assert_eq!(f("1234567.891", "N"), "1,234,567.89");
        assert_eq!(f("1234567.891", "N0"), "1,234,568");
        assert_eq!(f("-1234.5", "N1"), "-1,234.5");
        assert_eq!(f("123", "N2"), "123.00");
        assert_eq!(f("0", "N"), "0.00");
        assert_eq!(us("1234.5678", "N"), "1,234.568");
        assert_eq!(f("999999.999", "N2"), "1,000,000.00");

        let negative = |pattern| {
            d("-1234.5").to_string_with("N1", Some(&NumberFormatInfo::new().with_number_negative_pattern(pattern))).unwrap()
        };
        assert_eq!(negative(0), "(1,234.5)");
        assert_eq!(negative(1), "-1,234.5");
        assert_eq!(negative(2), "- 1,234.5");
        assert_eq!(negative(3), "1,234.5-");
        assert_eq!(negative(4), "1,234.5 -");

        let grouped = |sizes: &[i32]| {
            let info = NumberFormatInfo::new().with_number_group_sizes(sizes).with_number_group_separator(" ");
            d("123456789012").to_string_with("N0", Some(&info)).unwrap()
        };
        assert_eq!(grouped(&[3]), "123 456 789 012");
        assert_eq!(grouped(&[3, 2]), "1 23 45 67 89 012");
        assert_eq!(grouped(&[3, 0]), "123456789 012");
        assert_eq!(grouped(&[0]), "123456789012");
        assert_eq!(grouped(&[]), "123456789012");
    }

    #[test]
    fn currency_format() {
        assert_eq!(f("10.11", "C2"), "\u{00A4}10.11");
        assert_eq!(f("1234.565", "C"), "\u{00A4}1,234.57");
        assert_eq!(f("-1234.5", "C"), "(\u{00A4}1,234.50)");
        assert_eq!(f("1234.5", "C0"), "\u{00A4}1,235");
        assert_eq!(us("1234.5", "C"), "$1,234.50");
        assert_eq!(us("-1234.5", "c"), "-$1,234.50");

        let info = NumberFormatInfo::new()
            .with_currency_symbol("kr")
            .with_currency_positive_pattern(3)
            .with_currency_negative_pattern(8)
            .with_currency_decimal_separator(",")
            .with_currency_group_separator(".");
        assert_eq!(d("1234.5").to_string_with("C", Some(&info)).unwrap(), "1.234,50 kr");
        assert_eq!(d("-1234.5").to_string_with("C", Some(&info)).unwrap(), "-1.234,50 kr");
    }

    #[test]
    fn percent_format() {
        assert_eq!(f("10.11", "P2"), "1,011.00 %");
        assert_eq!(f("0.125", "P"), "12.50 %");
        assert_eq!(f("0.125", "P0"), "13 %");
        assert_eq!(f("-0.125", "P1"), "-12.5 %");
        assert_eq!(f("0", "P"), "0.00 %");
        assert_eq!(us("0.123456", "P"), "12.346%");
        assert_eq!(us("-0.5", "P0"), "-50%");
    }

    #[test]
    fn exponential_format() {
        assert_eq!(f("12345.6789", "E"), "1.234568E+004");
        assert_eq!(f("12345.6789", "e2"), "1.23e+004");
        assert_eq!(f("-0.00012", "E1"), "-1.2E-004");
        assert_eq!(f("0", "E"), "0.000000E+000");
        assert_eq!(f("5", "E0"), "5E+000");
        assert_eq!(f("9.99", "E1"), "1.0E+001");
    }

    #[test]
    fn invalid_format_specifiers() {
        let invariant = NumberFormatInfo::invariant_info();
        for format in ["D", "X", "x8", "B", "Q", "Z3", "F1000000000"] {
            assert_eq!(
                d("1").to_string_with(format, Some(&invariant)),
                Err(FormatError::InvalidFormatSpecifier),
                "{format}"
            );
        }
    }

    #[test]
    fn custom_formats() {
        assert_eq!(f("10.11", "0.00"), "10.11");
        assert_eq!(f("10.115", "0.00"), "10.12");
        assert_eq!(f("10.1", "0.00"), "10.10");
        assert_eq!(f("10.1", "0.##"), "10.1");
        assert_eq!(f("10", "0.##"), "10");
        assert_eq!(f("0.5", "#.#"), ".5");
        assert_eq!(f("-0.5", "#.#"), "-.5");
        assert_eq!(f("0", "#"), "");
        assert_eq!(f("0", "0"), "0");
        assert_eq!(f("5", "000"), "005");
        assert_eq!(f("12345", "00"), "12345");
        assert_eq!(f("2.5", "0"), "3");
        assert_eq!(f("-2.5", "0"), "-3");
        assert_eq!(f("-0.4", "0"), "0");
        assert_eq!(f("1234567.891", "#,##0.00"), "1,234,567.89");
        assert_eq!(f("1234567.891", "#,#"), "1,234,568");
        assert_eq!(f("123", "#,##0"), "123");
        assert_eq!(f("5", "0,000"), "0,005");
        assert_eq!(f("1234567", "0,"), "1235");
        assert_eq!(f("1234567", "0,,"), "1");
        assert_eq!(f("1234567", "#,##0,"), "1,235");
        assert_eq!(f("0.1234", "0.0%"), "12.3%");
        assert_eq!(f("0.1234", "0.0\u{2030}"), "123.4\u{2030}");
        assert_eq!(f("90", "0 \u{00B0}"), "90 \u{00B0}");
        assert_eq!(f("90", "0' deg'"), "90 deg");
        assert_eq!(f("90", "0\" P\""), "90 P");
        assert_eq!(f("90", "\\#0"), "#90");
        assert_eq!(f("12", "0.0.0"), "12.00");
        assert_eq!(f("1.5", "abc"), "abc");
        // Sections: positive; negative; zero.
        assert_eq!(f("5", "+0;-0;zero"), "+5");
        assert_eq!(f("-5", "+0;-0;zero"), "-5");
        assert_eq!(f("0", "+0;-0;zero"), "zero");
        assert_eq!(f("-5", "0;(0)"), "(5)");
        assert_eq!(f("0", "0;(0)"), "0");
        assert_eq!(f("0.001", "0.0;(0.0);nil"), "nil");
        assert_eq!(f("-0.001", "0.0;(0.0)"), "0.0");
        assert_eq!(f("-5", "0;;zero"), "-5");
        // Exponents.
        assert_eq!(f("12345", "0.00E+0"), "1.23E+4");
        assert_eq!(f("12345", "0.00E+000"), "1.23E+004");
        assert_eq!(f("0.00012", "0.0e-0"), "1.2e-4");
        assert_eq!(f("12345", "0.0E0"), "1.2E4");
        assert_eq!(f("12345", "##0.0E+0"), "123.5E+2");
        assert_eq!(f("5", "0E"), "5E");
        assert_eq!(f("5", "0 E+"), "5 E+");

        let info = NumberFormatInfo::new().with_number_decimal_separator(",").with_number_group_separator(".");
        assert_eq!(d("1234.5").to_string_with("#,##0.00", Some(&info)).unwrap(), "1.234,50");
        let info = NumberFormatInfo::new().with_number_group_sizes(&[3, 2]);
        assert_eq!(d("123456789").to_string_with("#,0", Some(&info)).unwrap(), "12,34,56,789");
    }

    #[test]
    fn composite_formats() {
        let invariant = NumberFormatInfo::invariant_info();
        let format = |format: &str, value: Option<&str>| format_decimal_composite(format, &invariant, value.map(d));
        assert_eq!(format("{0:N2} \u{00B0}", Some("90")).unwrap(), "90.00 \u{00B0}");
        assert_eq!(format("{0}", Some("1.10")).unwrap(), "1.10");
        assert_eq!(format("[{0,8:F1}]", Some("1.25")).unwrap(), "[     1.3]");
        assert_eq!(format("[{0,-6}]", Some("1.5")).unwrap(), "[1.5   ]");
        assert_eq!(format("{{{0}}}", Some("7")).unwrap(), "{7}");
        assert_eq!(format("{0} and {0:0.0}", Some("2")).unwrap(), "2 and 2.0");
        assert_eq!(format("{0:N2} x", None).unwrap(), " x");
        assert_eq!(format("no items", Some("1")).unwrap(), "no items");
        assert_eq!(format("{1}", Some("1")), Err(FormatError::IndexOutOfRange));
        assert_eq!(format("{0", Some("1")), Err(FormatError::InvalidFormat));
        assert_eq!(format("}", Some("1")), Err(FormatError::InvalidFormat));
        assert_eq!(format("{x}", Some("1")), Err(FormatError::InvalidFormat));
        assert_eq!(format("{0:Q}", Some("1")), Err(FormatError::InvalidFormatSpecifier));
    }

    fn p(text: &str, styles: NumberStyles) -> Result<String, NumberParseError> {
        Decimal::parse_with(text, styles, Some(&NumberFormatInfo::invariant_info())).map(|value| {
            if value.is_sign_negative() && value.is_zero() {
                format!("-{value}")
            } else {
                value.to_string()
            }
        })
    }

    #[test]
    fn parse_with_styles() {
        let format = Err(NumberParseError::Format);

        assert_eq!(p("123", NumberStyles::NONE).unwrap(), "123");
        assert_eq!(p(" 123", NumberStyles::NONE), format);
        assert_eq!(p("-123", NumberStyles::NONE), format);
        assert_eq!(p("1.5", NumberStyles::NONE), format);
        assert_eq!(p("", NumberStyles::ANY), format);
        assert_eq!(p("  ", NumberStyles::ANY), format);
        assert_eq!(p("-", NumberStyles::ANY), format);
        assert_eq!(p(".", NumberStyles::ANY), format);

        assert_eq!(p("  123  ", NumberStyles::INTEGER).unwrap(), "123");
        assert_eq!(p("\t-123\r\n", NumberStyles::INTEGER).unwrap(), "-123");
        assert_eq!(p("+123", NumberStyles::INTEGER).unwrap(), "123");
        assert_eq!(p("123-", NumberStyles::INTEGER), format);
        assert_eq!(p("- 123", NumberStyles::INTEGER), format);
        assert_eq!(p("1,234", NumberStyles::INTEGER), format);

        assert_eq!(p("1,234.50", NumberStyles::NUMBER).unwrap(), "1234.50");
        assert_eq!(p("1,2,3,4", NumberStyles::NUMBER).unwrap(), "1234");
        assert_eq!(p(",123", NumberStyles::NUMBER), format);
        assert_eq!(p("1.2,3", NumberStyles::NUMBER), format);
        assert_eq!(p("123-", NumberStyles::NUMBER).unwrap(), "-123");
        assert_eq!(p("123 -", NumberStyles::NUMBER).unwrap(), "-123");
        assert_eq!(p("-123-", NumberStyles::NUMBER), format);
        assert_eq!(p("1.2.3", NumberStyles::NUMBER), format);
        assert_eq!(p("(123)", NumberStyles::NUMBER), format);
        assert_eq!(p("0.00", NumberStyles::NUMBER).unwrap(), "0.00");
        assert_eq!(p("-0.0", NumberStyles::NUMBER).unwrap(), "-0.0");
        assert_eq!(p("000123.4500", NumberStyles::NUMBER).unwrap(), "123.4500");
        assert_eq!(p("123\0\0", NumberStyles::NUMBER).unwrap(), "123");
        assert_eq!(p("123x", NumberStyles::NUMBER), format);

        assert_eq!(p("1e2", NumberStyles::FLOAT).unwrap(), "100");
        assert_eq!(p("1.5E-1", NumberStyles::FLOAT).unwrap(), "0.15");
        assert_eq!(p("1.5e+3", NumberStyles::FLOAT).unwrap(), "1500");
        assert_eq!(p("12e", NumberStyles::FLOAT), format);
        assert_eq!(p("1e-30", NumberStyles::FLOAT).unwrap(), "0.0000000000000000000000000000");
        assert_eq!(p("1e29", NumberStyles::FLOAT), Err(NumberParseError::Overflow));
        assert_eq!(p("1e99999", NumberStyles::FLOAT), Err(NumberParseError::Overflow));
        assert_eq!(p("1e2", NumberStyles::NUMBER), format);

        assert_eq!(p("(123.4)", NumberStyles::CURRENCY).unwrap(), "-123.4");
        assert_eq!(p("(123.4", NumberStyles::CURRENCY), format);
        assert_eq!(p("\u{00A4}123", NumberStyles::CURRENCY).unwrap(), "123");
        assert_eq!(p("123\u{00A4}", NumberStyles::CURRENCY).unwrap(), "123");
        assert_eq!(p("-\u{00A4} 123", NumberStyles::CURRENCY).unwrap(), "-123");
        assert_eq!(p("(\u{00A4}1,234.50)", NumberStyles::CURRENCY).unwrap(), "-1234.50");
        assert_eq!(p("\u{00A4}\u{00A4}123", NumberStyles::CURRENCY), format);
        assert_eq!(p("\u{00A4}123", NumberStyles::NUMBER), format);
        assert_eq!(p(" (1e1) ", NumberStyles::ANY).unwrap(), "-10");

        assert_eq!(p("FF", NumberStyles::HEX_NUMBER), Err(NumberParseError::InvalidStyle));
        assert_eq!(p("11", NumberStyles::BINARY_NUMBER), Err(NumberParseError::InvalidStyle));
        assert_eq!(p("1", NumberStyles::from_bits_retain(0x1000)), Err(NumberParseError::InvalidStyle));
        assert_eq!(
            Decimal::try_parse_with("12", NumberStyles::ANY, Some(&NumberFormatInfo::invariant_info())),
            Some(d("12"))
        );
        assert_eq!(Decimal::try_parse_with("x", NumberStyles::ANY, None), None);
    }

    #[test]
    fn parse_with_cultures() {
        let english = english();
        let parse = |text: &str, styles| Decimal::parse_with(text, styles, Some(&english)).map(|v| v.to_string());
        assert_eq!(parse("$1,234.50", NumberStyles::CURRENCY).unwrap(), "1234.50");
        assert_eq!(parse("-$5", NumberStyles::CURRENCY).unwrap(), "-5");
        assert_eq!(parse("\u{00A4}5", NumberStyles::CURRENCY), Err(NumberParseError::Format));

        let info = NumberFormatInfo::new()
            .with_number_decimal_separator(",")
            .with_number_group_separator("\u{00A0}")
            .with_negative_sign("\u{2212}")
            .with_currency_decimal_separator(";")
            .with_currency_group_separator("'")
            .with_currency_symbol("kr");
        let parse = |text: &str, styles| Decimal::parse_with(text, styles, Some(&info)).map(|v| v.to_string());
        assert_eq!(parse("1\u{00A0}234,5", NumberStyles::NUMBER).unwrap(), "1234.5");
        // A plain space matches a no-break space of the format information.
        assert_eq!(parse("1 234,5", NumberStyles::NUMBER).unwrap(), "1234.5");
        assert_eq!(parse("1.5", NumberStyles::NUMBER), Err(NumberParseError::Format));
        // A hyphen matches the minus sign of the format information.
        assert_eq!(parse("\u{2212}5", NumberStyles::NUMBER).unwrap(), "-5");
        assert_eq!(parse("-5", NumberStyles::NUMBER).unwrap(), "-5");
        // The currency separators, and the number separators until a
        // currency symbol was seen.
        assert_eq!(parse("1'234;5 kr", NumberStyles::CURRENCY).unwrap(), "1234.5");
        assert_eq!(parse("1 234,5 kr", NumberStyles::CURRENCY).unwrap(), "1234.5");
        assert_eq!(parse("kr 1 234,5", NumberStyles::CURRENCY), Err(NumberParseError::Format));

        // Text formatted with a culture parses back with it.
        let text = d("-1234.5").to_string_with("N2", Some(&info)).unwrap();
        assert_eq!(text, "\u{2212}1\u{00A0}234,50");
        assert_eq!(parse(&text, NumberStyles::NUMBER).unwrap(), "-1234.50");

        // Without a provider the current culture is used.
        let _scope = FerroLocator::enter_scope();
        TestCultureDataProvider::register();
        let previous = CultureInfo::current_culture();
        CultureInfo::set_current_culture(CultureInfo::get_culture_info("en-US"));
        assert_eq!(d("0.5").to_string_with("P0", None).unwrap(), "50%");
        assert_eq!(Decimal::parse_with("$5", NumberStyles::CURRENCY, None), Ok(d("5")));
        CultureInfo::set_current_culture(previous);
    }
}
