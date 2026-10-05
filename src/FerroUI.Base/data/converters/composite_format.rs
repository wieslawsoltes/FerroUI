//! Composite formatting: the `string.Format` of the managed runtime.
//! [`format`] and [`format_arg`] use the invariant culture, the `_with`
//! forms the number and date formats of a [`CultureInfo`] and
//! [`format_values`] those of the current culture.
//!
//! A composite format string is literal text with format items
//! `{index[,alignment][:formatString]}`. `{{` and `}}` are literal braces.
//! The results match the managed runtime (version 10) character for
//! character for the supported argument types.
//!
//! # Supported
//!
//! - Format items: index, repeated and out-of-order indices, alignment
//!   (`{0,10}` right-aligns, `{0,-10}` left-aligns; the width counts UTF-16
//!   code units), format strings, `{{` / `}}` escapes, spaces after the index
//!   and around the alignment. A `}` ends the format string of an item and a
//!   `{` inside it is an error, as in the managed runtime.
//! - Arguments: null (empty text), strings, booleans (`True` / `False`),
//!   characters, signed and unsigned integers and floating point numbers.
//!   Strings, booleans and characters ignore the format string.
//! - Default number formatting (no format string): integers as decimal
//!   digits; floating point numbers as the shortest text that round-trips,
//!   switching to exponent notation (`1E+17`, `1E-05`) when the value is
//!   below `0.0001` or its integer part has more digits than both the text
//!   and the round-trip precision (17 for `f64`, 9 for `f32`); `NaN`,
//!   `Infinity`, `-Infinity`, `-0`.
//! - Standard numeric format strings, upper and lower case, with an optional
//!   precision of up to 999,999,999: `C` (currency symbol `¤`, negative in
//!   parentheses), `D` (integers only), `E`, `F`, `G`, `N`, `P` (`12.50 %`),
//!   `R`, `X` (integers only; negative values in two's complement of the
//!   integer's width), `B` (integers only).
//! - Custom numeric format strings: `0`, `#`, `.`, `,` as group separator and
//!   as number scaling, `%`, `‰`, `;` sections (positive; negative; zero),
//!   `E0` / `E+0` / `E-0` / `e0` / `e+0` / `e-0` exponents, quoted literals
//!   (`'..'`, `".."`), `\` escapes; every other character is copied.
//! - Rounding: standard format strings round floating point numbers
//!   correctly from the exact binary value, exact midpoints going to the even
//!   digit (`0.125` with `F2` is `0.12`, `2.5` with `F0` is `2`); custom
//!   format strings first take 15 (`f64`) or 7 (`f32`) significant digits and
//!   then round midpoints away from zero (`2.5` with `0` is `3`); integers
//!   round midpoints away from zero (`12350` with `E2` is `1.24E+004`).
//!
//! - Decimals (standard and custom numeric format strings), dates and
//!   times (`DateTime`, `DateTimeOffset`: standard and custom date and time
//!   format strings) and time spans, formatted by their own types with the
//!   format string of the item.
//! - Cultures: decimal and group separators, group sizes, the negative and
//!   positive signs, the currency and percent symbols and patterns, the
//!   default numbers of decimal digits and the symbols of the values that
//!   are not numbers come from the `NumberFormatInfo` of the culture; dates
//!   use its `DateTimeFormatInfo`.
//!
//! # Not supported
//!
//! - Custom format providers.
//! - GUID and enumeration format strings and arbitrary precision numbers.
//!   Values of such types are formatted through their registered display
//!   form and ignore the format string of the item. The `g` / `G` formats
//!   of a time span use `.` as the decimal separator in every culture.
//! - 128-bit integers and half precision floats.

use crate::data::core::ValueTypes;
use crate::data::BindingNotification;
use crate::utilities::number_format::{
    pattern, NEGATIVE_CURRENCY_FORMATS, NEGATIVE_NUMBER_FORMATS, NEGATIVE_PERCENT_FORMATS, POSITIVE_CURRENCY_FORMATS,
    POSITIVE_PERCENT_FORMATS,
};
use crate::animation::TimeSpan;
use crate::utilities::{CultureInfo, DateTime, DateTimeOffset, Decimal, NumberFormatInfo};
use crate::{BoxedValue, DoNothingType, UnsetValueType};
use std::fmt;

/// An argument of a composite format operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormatArg<'a> {
    /// A null reference: formats as empty text.
    Null,
    /// A string: ignores the format string of the item.
    Str(&'a str),
    /// A boolean: `True` or `False`.
    Bool(bool),
    /// A character.
    Char(char),
    /// An 8-bit signed integer (the width matters for `X` and `B`).
    I8(i8),
    /// A 16-bit signed integer (the width matters for `X` and `B`).
    I16(i16),
    /// A 32-bit signed integer (the width matters for `X` and `B`).
    I32(i32),
    /// A 64-bit signed integer.
    I64(i64),
    /// An unsigned integer of any width.
    U64(u64),
    /// A double precision floating point number.
    F64(f64),
    /// A single precision floating point number.
    F32(f32),
    /// A decimal (128-bit) number.
    Decimal(Decimal),
    /// A date and time: formatted with date and time format strings.
    DateTime(DateTime),
    /// A date and time with an offset from UTC.
    DateTimeOffset(DateTimeOffset),
    /// A time interval: formatted with time span format strings.
    TimeSpan(TimeSpan),
}

/// The error of a composite format operation: the equivalent of the format
/// exception of the managed runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FormatError {
    /// The composite format string is malformed: unbalanced braces or a
    /// malformed format item.
    InvalidFormat,
    /// A format item refers to an argument that does not exist.
    IndexOutOfRange,
    /// A numeric format string is not valid for the argument.
    InvalidFormatSpecifier,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FormatError::InvalidFormat => "Input string was not in a correct format.",
            FormatError::IndexOutOfRange => {
                "Index (zero based) must be greater than or equal to zero and less than the size of the argument list."
            }
            FormatError::InvalidFormatSpecifier => "Format specifier was invalid.",
        })
    }
}

impl std::error::Error for FormatError {}

const INDEX_LIMIT: usize = 1_000_000;
const WIDTH_LIMIT: usize = 1_000_000;

/// Replaces the format items of `format` with the text of the matching
/// arguments.
pub fn format(format: &str, args: &[FormatArg<'_>]) -> Result<String, FormatError> {
    format_with(format, args, &CultureInfo::invariant_culture())
}

/// Replaces the format items of `format` with the text of the matching
/// arguments, formatted with the conventions of `culture`: the
/// `string.Format(IFormatProvider, ..)` of the managed runtime.
pub fn format_with(format: &str, args: &[FormatArg<'_>], culture: &CultureInfo) -> Result<String, FormatError> {
    fn next(bytes: &[u8], pos: &mut usize) -> Result<u8, FormatError> {
        *pos += 1;
        bytes.get(*pos).copied().ok_or(FormatError::InvalidFormat)
    }

    let bytes = format.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + args.len() * 8);
    let mut pos = 0;
    loop {
        // Literal text up to the next brace. Braces are ASCII, so every
        // position the scan stops at is a character boundary.
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
        if brace != b'{' {
            return Err(FormatError::InvalidFormat);
        }

        // The argument index.
        if !ch.is_ascii_digit() {
            return Err(FormatError::InvalidFormat);
        }
        let mut index = usize::from(ch - b'0');
        ch = next(bytes, &mut pos)?;
        let mut width = 0usize;
        let mut left_justify = false;
        let mut item_format = "";
        if ch != b'}' {
            while ch.is_ascii_digit() && index < INDEX_LIMIT {
                index = index * 10 + usize::from(ch - b'0');
                ch = next(bytes, &mut pos)?;
            }
            while ch == b' ' {
                ch = next(bytes, &mut pos)?;
            }

            // The alignment.
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
                while ch.is_ascii_digit() && width < WIDTH_LIMIT {
                    width = width * 10 + usize::from(ch - b'0');
                    ch = next(bytes, &mut pos)?;
                }
                while ch == b' ' {
                    ch = next(bytes, &mut pos)?;
                }
            }

            // The format string.
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

        let arg = args.get(index).ok_or(FormatError::IndexOutOfRange)?;
        if width == 0 {
            write_arg(&mut out, arg, item_format, culture)?;
        } else {
            let mut text = String::new();
            write_arg(&mut text, arg, item_format, culture)?;
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
}

/// Formats one argument with a numeric format string: the equivalent of
/// `ToString(format)` with the invariant culture.
pub fn format_arg(arg: &FormatArg<'_>, format: &str) -> Result<String, FormatError> {
    format_arg_with(arg, format, &CultureInfo::invariant_culture())
}

/// Formats one argument with a format string and the conventions of
/// `culture`: the equivalent of `ToString(format, culture)`.
pub fn format_arg_with(arg: &FormatArg<'_>, format: &str, culture: &CultureInfo) -> Result<String, FormatError> {
    let mut out = String::new();
    write_arg(&mut out, arg, format, culture)?;
    Ok(out)
}

/// Replaces the format items of `format` with the text of the matching
/// untyped values.
///
/// Nullable (`Option<T>`) values are unwrapped first. Integers, floating
/// point numbers, booleans, characters and strings are formatted as the
/// matching [`FormatArg`]; every other value is formatted as its display form
/// ([`ValueTypes::to_display_string`]) and ignores the format string of the
/// item.
///
/// Numbers, decimals and dates are formatted with the conventions of the
/// current culture (`string.Format` without a provider); see
/// [`format_values_with`] for another culture.
pub fn format_values(format_string: &str, values: &[Option<BoxedValue>]) -> Result<String, FormatError> {
    format_values_with(format_string, values, &CultureInfo::current_culture())
}

/// [`format_values`] with the conventions of `culture`.
pub fn format_values_with(
    format_string: &str,
    values: &[Option<BoxedValue>],
    culture: &CultureInfo,
) -> Result<String, FormatError> {
    enum Owned {
        Arg(FormatArg<'static>),
        Text(String),
    }

    fn to_owned(value: &Option<BoxedValue>) -> Owned {
        let Some(value) = value.clone().and_then(ValueTypes::normalize) else {
            return Owned::Arg(FormatArg::Null);
        };
        macro_rules! primitive {
            ($($ty:ty => |$v:ident| $arg:expr),* $(,)?) => {
                $(
                    if let Some($v) = value.downcast_ref::<$ty>() {
                        return Owned::Arg($arg);
                    }
                )*
            };
        }
        primitive!(
            i32 => |v| FormatArg::I32(*v),
            f64 => |v| FormatArg::F64(*v),
            bool => |v| FormatArg::Bool(*v),
            i64 => |v| FormatArg::I64(*v),
            f32 => |v| FormatArg::F32(*v),
            u8 => |v| FormatArg::U64(u64::from(*v)),
            u16 => |v| FormatArg::U64(u64::from(*v)),
            u32 => |v| FormatArg::U64(u64::from(*v)),
            u64 => |v| FormatArg::U64(*v),
            usize => |v| FormatArg::U64(*v as u64),
            i8 => |v| FormatArg::I8(*v),
            i16 => |v| FormatArg::I16(*v),
            isize => |v| FormatArg::I64(*v as i64),
            char => |v| FormatArg::Char(*v),
            Decimal => |v| FormatArg::Decimal(*v),
            DateTime => |v| FormatArg::DateTime(*v),
            DateTimeOffset => |v| FormatArg::DateTimeOffset(*v),
            TimeSpan => |v| FormatArg::TimeSpan(*v),
            &'static str => |v| FormatArg::Str(v),
        );
        if let Some(s) = value.downcast_ref::<String>() {
            return Owned::Text(s.clone());
        }
        if let Some(v) = value.downcast_ref::<UnsetValueType>() {
            return Owned::Text(v.to_string());
        }
        if let Some(v) = value.downcast_ref::<DoNothingType>() {
            return Owned::Text(v.to_string());
        }
        if let Some(v) = value.downcast_ref::<BindingNotification>() {
            return Owned::Text(v.to_string());
        }
        Owned::Text(ValueTypes::to_display_string(Some(&value)))
    }

    let owned: Vec<Owned> = values.iter().map(to_owned).collect();
    let args: Vec<FormatArg<'_>> = owned
        .iter()
        .map(|o| match o {
            Owned::Arg(arg) => *arg,
            Owned::Text(text) => FormatArg::Str(text),
        })
        .collect();
    format_with(format_string, &args, culture)
}

fn write_arg(out: &mut String, arg: &FormatArg<'_>, format: &str, culture: &CultureInfo) -> Result<(), FormatError> {
    let numbers = culture.number_format();
    let info: &NumberFormatInfo = &numbers;
    match *arg {
        FormatArg::Null => {}
        FormatArg::Str(s) => out.push_str(s),
        FormatArg::Bool(b) => out.push_str(if b { "True" } else { "False" }),
        FormatArg::Char(c) => out.push(c),
        FormatArg::I8(v) => format_integer(out, v < 0, u64::from(v.unsigned_abs()), u64::from(v as u8), format, info)?,
        FormatArg::I16(v) => format_integer(out, v < 0, u64::from(v.unsigned_abs()), u64::from(v as u16), format, info)?,
        FormatArg::I32(v) => format_integer(out, v < 0, u64::from(v.unsigned_abs()), u64::from(v as u32), format, info)?,
        FormatArg::I64(v) => format_integer(out, v < 0, v.unsigned_abs(), v as u64, format, info)?,
        FormatArg::U64(v) => format_integer(out, false, v, v, format, info)?,
        FormatArg::F64(v) => format_float(out, Float::F64(v), format, info)?,
        FormatArg::F32(v) => format_float(out, Float::F32(v), format, info)?,
        FormatArg::Decimal(v) => out.push_str(&v.to_string_with(format, Some(info))?),
        // An invalid date and time format string is the format exception.
        FormatArg::DateTime(v) => {
            out.push_str(&v.try_to_string_format(format, culture).map_err(|_| FormatError::InvalidFormat)?)
        }
        FormatArg::DateTimeOffset(v) => {
            out.push_str(&v.try_to_string_format(format, culture).map_err(|_| FormatError::InvalidFormat)?)
        }
        FormatArg::TimeSpan(v) => {
            out.push_str(&v.try_to_string_format(format).map_err(|_| FormatError::InvalidFormat)?)
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Numbers
// ---------------------------------------------------------------------------

/// A number as decimal digits: `0.d1d2d3.. * 10^scale`. Zero has no digits
/// and scale 0.
struct Number {
    /// ASCII digits.
    digits: Vec<u8>,
    scale: i32,
    negative: bool,
    /// A floating point number: keeps its sign when it is (or rounds to)
    /// zero, and has been rounded correctly already for standard formats.
    floating: bool,
}

impl Number {
    fn from_integer(negative: bool, abs: u64) -> Self {
        let digits = if abs == 0 { Vec::new() } else { abs.to_string().into_bytes() };
        let scale = digits.len() as i32;
        // Trailing zeros stay: the digit count is the default precision of
        // the general format.
        Number { digits, scale, negative, floating: false }
    }

    /// From the output of exponent formatting: `d.ddd..e[-]x`.
    fn from_exponent_text(text: &str, negative: bool) -> Self {
        let (mantissa, exponent) = text.split_once('e').unwrap_or((text, "0"));
        let exponent: i32 = exponent.parse().unwrap_or(0);
        let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
        let mut number = Number { digits, scale: exponent + 1, negative, floating: true };
        number.trim();
        if number.digits.is_empty() {
            number.scale = 0;
        }
        number
    }

    /// From the output of fixed formatting: `ddd.ddd`.
    fn from_fixed_text(text: &str, negative: bool) -> Self {
        let integer_len = text.find('.').unwrap_or(text.len());
        let all: Vec<u8> = text.bytes().filter(u8::is_ascii_digit).collect();
        let leading_zeros = all.iter().take_while(|&&d| d == b'0').count();
        let digits = all[leading_zeros..].to_vec();
        let mut number =
            Number { digits, scale: integer_len as i32 - leading_zeros as i32, negative, floating: true };
        number.trim();
        if number.digits.is_empty() {
            number.scale = 0;
        }
        number
    }

    fn trim(&mut self) {
        while self.digits.last() == Some(&b'0') {
            self.digits.pop();
        }
    }

    /// The digit at `index`, `'0'` after the last digit.
    #[inline]
    fn digit(&self, index: usize) -> char {
        self.digits.get(index).map_or('0', |&d| d as char)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }
}

/// Rounds a number to `pos` digits. Numbers that were rounded correctly from
/// their exact value are only truncated; others round midpoints up.
fn round_number(number: &mut Number, pos: i64, is_correctly_rounded: bool) {
    let len = number.digits.len();
    let mut i = if pos <= 0 { 0 } else { len.min(usize::try_from(pos).unwrap_or(usize::MAX)) };
    let at_pos = pos >= 0 && i as i64 == pos;
    let round_up = at_pos && i < len && !is_correctly_rounded && number.digits[i] >= b'5';
    if round_up {
        while i > 0 && number.digits[i - 1] == b'9' {
            i -= 1;
        }
        if i > 0 {
            number.digits[i - 1] += 1;
        } else {
            number.scale += 1;
            if number.digits.is_empty() {
                number.digits.push(b'1');
            } else {
                number.digits[0] = b'1';
            }
            i = 1;
        }
    } else {
        while i > 0 && number.digits[i - 1] == b'0' {
            i -= 1;
        }
    }
    if i == 0 {
        if !number.floating {
            number.negative = false;
        }
        number.scale = 0;
    }
    number.digits.truncate(i);
}

enum Specifier {
    /// A standard format: the specifier and its precision (-1 if absent).
    Standard(u8, i32),
    Custom,
}

fn parse_format_specifier(format: &str) -> Result<Specifier, FormatError> {
    let bytes = format.as_bytes();
    let Some(&c) = bytes.first() else {
        return Ok(Specifier::Standard(b'G', -1));
    };
    if c.is_ascii_alphabetic() {
        let mut n: i32 = 0;
        let mut i = 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            if n >= 100_000_000 {
                return Err(FormatError::InvalidFormatSpecifier);
            }
            n = n * 10 + i32::from(bytes[i] - b'0');
            i += 1;
        }
        if i == bytes.len() || bytes[i] == 0 {
            return Ok(Specifier::Standard(c, if i == 1 { -1 } else { n }));
        }
    }
    Ok(if c == 0 { Specifier::Standard(b'G', -1) } else { Specifier::Custom })
}

fn push_zeros(out: &mut String, count: usize) {
    out.extend(std::iter::repeat_n('0', count));
}

/// `twos` is the two's complement bit pattern of the value in its own width.
fn format_integer(
    out: &mut String,
    negative: bool,
    abs: u64,
    twos: u64,
    format: &str,
    info: &NumberFormatInfo,
) -> Result<(), FormatError> {
    let (specifier, precision) = match parse_format_specifier(format)? {
        Specifier::Standard(specifier, precision) => (specifier, precision),
        Specifier::Custom => {
            let mut number = Number::from_integer(negative, abs);
            let format: Vec<char> = format.chars().collect();
            number_to_string_format(out, &mut number, &format, info);
            return Ok(());
        }
    };
    let min_digits = usize::try_from(precision).unwrap_or(0);
    let padded = |out: &mut String, digits: &str| {
        push_zeros(out, min_digits.saturating_sub(digits.len()));
        out.push_str(digits);
    };
    match specifier.to_ascii_uppercase() {
        b'G' if precision < 1 => {
            if negative {
                out.push_str(info.negative_sign());
            }
            out.push_str(&abs.to_string());
        }
        b'D' => {
            if negative {
                out.push_str(info.negative_sign());
            }
            padded(out, &abs.to_string());
        }
        b'X' => {
            let digits = if specifier == b'X' { format!("{twos:X}") } else { format!("{twos:x}") };
            padded(out, &digits);
        }
        b'B' => padded(out, &format!("{twos:b}")),
        _ => {
            let mut number = Number::from_integer(negative, abs);
            number_to_string(out, &mut number, specifier, precision, info)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Float {
    F64(f64),
    F32(f32),
}

impl Float {
    fn is_nan(self) -> bool {
        match self {
            Float::F64(v) => v.is_nan(),
            Float::F32(v) => v.is_nan(),
        }
    }

    fn is_infinite(self) -> bool {
        match self {
            Float::F64(v) => v.is_infinite(),
            Float::F32(v) => v.is_infinite(),
        }
    }

    fn is_negative(self) -> bool {
        match self {
            Float::F64(v) => v.is_sign_negative(),
            Float::F32(v) => v.is_sign_negative(),
        }
    }

    fn is_zero(self) -> bool {
        match self {
            Float::F64(v) => v == 0.0,
            Float::F32(v) => v == 0.0,
        }
    }

    /// The number of significant digits custom formats work with.
    fn custom_format_precision(self) -> i32 {
        match self {
            Float::F64(_) => 15,
            Float::F32(_) => 7,
        }
    }

    /// The number of digits that always round-trip.
    fn max_round_trip_digits(self) -> i32 {
        match self {
            Float::F64(_) => 17,
            Float::F32(_) => 9,
        }
    }

    /// The shortest digits that round-trip. When the exact value lies halfway
    /// between two candidates of that length the one with the even last digit
    /// is used, provided it round-trips.
    fn shortest(self) -> Number {
        // The managed runtime computes the lower rounding boundary of powers
        // of two as if it were as far away as the upper one. Of all `f64` and
        // `f32` powers of two this changes the result of exactly two values,
        // which get one digit less than needed to round-trip.
        if let Float::F64(v) = self {
            let quirk = match v.abs().to_bits() {
                0x3E60_0000_0000_0000 => Some("2.980232238769531e-8"), // 2^-25
                0x0410_0000_0000_0000 => Some("4.104536801298376e-289"), // 2^-958
                _ => None,
            };
            if let Some(text) = quirk {
                return Number::from_exponent_text(text, self.is_negative());
            }
        }
        let text = match self {
            Float::F64(v) => format!("{:e}", v.abs()),
            Float::F32(v) => format!("{:e}", v.abs()),
        };
        let shortest = Number::from_exponent_text(&text, self.is_negative());
        let mantissa_len = text.find('e').unwrap_or(text.len());
        let decimals = mantissa_len.saturating_sub(2);
        let rounded_text = match self {
            Float::F64(v) => format!("{:.*e}", decimals, v.abs()),
            Float::F32(v) => format!("{:.*e}", decimals, v.abs()),
        };
        if rounded_text == text {
            return shortest;
        }
        let round_trips = match self {
            Float::F64(v) => rounded_text.parse::<f64>().is_ok_and(|r| r == v.abs()),
            Float::F32(v) => rounded_text.parse::<f32>().is_ok_and(|r| r == v.abs()),
        };
        if round_trips {
            Number::from_exponent_text(&rounded_text, self.is_negative())
        } else {
            shortest
        }
    }

    /// The exact value rounded to `count` significant digits.
    fn significant_digits(self, count: i32) -> Number {
        // No value has more significant digits than this: more are zeros.
        let decimals = (count.clamp(1, 800) - 1) as usize;
        let text = match self {
            Float::F64(v) => format!("{:.*e}", decimals, v.abs()),
            Float::F32(v) => format!("{:.*e}", decimals, v.abs()),
        };
        Number::from_exponent_text(&text, self.is_negative())
    }

    /// The exact value rounded to `count` digits after the decimal point.
    fn decimal_digits(self, count: i32) -> Number {
        // No value has more decimal digits than this: more are zeros.
        let decimals = count.clamp(0, 1100) as usize;
        let text = match self {
            Float::F64(v) => format!("{:.*}", decimals, v.abs()),
            Float::F32(v) => format!("{:.*}", decimals, v.abs()),
        };
        Number::from_fixed_text(&text, self.is_negative())
    }
}

fn format_float(out: &mut String, value: Float, format: &str, info: &NumberFormatInfo) -> Result<(), FormatError> {
    if value.is_nan() {
        out.push_str(info.nan_symbol());
        return Ok(());
    }
    if value.is_infinite() {
        out.push_str(if value.is_negative() {
            info.negative_infinity_symbol()
        } else {
            info.positive_infinity_symbol()
        });
        return Ok(());
    }

    let (specifier, precision) = match parse_format_specifier(format)? {
        Specifier::Standard(specifier, precision) => (specifier, precision),
        Specifier::Custom => {
            let mut number = if value.is_zero() {
                Number { digits: Vec::new(), scale: 0, negative: value.is_negative(), floating: true }
            } else {
                value.significant_digits(value.custom_format_precision())
            };
            let format: Vec<char> = format.chars().collect();
            number_to_string_format(out, &mut number, &format, info);
            return Ok(());
        }
    };

    enum Digits {
        Shortest,
        Significant(i32),
        Decimals(i32),
    }
    let or_default = |default: i32| if precision == -1 { default } else { precision };
    let digits = match specifier.to_ascii_uppercase() {
        b'C' => Digits::Decimals(or_default(info.currency_decimal_digits())),
        b'F' | b'N' => Digits::Decimals(or_default(info.number_decimal_digits())),
        b'E' => Digits::Significant(or_default(6).saturating_add(1)),
        b'G' if precision < 1 => Digits::Shortest,
        b'G' => Digits::Significant(precision),
        b'P' => Digits::Decimals(or_default(info.percent_decimal_digits()).saturating_add(2)),
        b'R' => Digits::Shortest,
        _ => return Err(FormatError::InvalidFormatSpecifier),
    };

    let mut max_digits = precision;
    let mut number = if value.is_zero() {
        Number { digits: Vec::new(), scale: 0, negative: value.is_negative(), floating: true }
    } else {
        match digits {
            Digits::Shortest => value.shortest(),
            Digits::Significant(count) => value.significant_digits(count),
            Digits::Decimals(count) => value.decimal_digits(count),
        }
    };
    if matches!(digits, Digits::Shortest) {
        // Keeps short values "pretty": without this 60 would be 6E+01.
        max_digits = (number.digits.len() as i32).max(value.max_round_trip_digits());
    }
    number_to_string(out, &mut number, specifier, max_digits, info)
}

// ---------------------------------------------------------------------------
// Standard numeric format strings
// ---------------------------------------------------------------------------

const PER_MILLE_SYMBOL: char = '\u{2030}';

/// Writes a number laid out by a pattern of the format information (`($#)`,
/// `-# %`): `#` is the digits, `-` the negative sign, `$` and `%` the
/// symbols; everything else is copied.
fn write_pattern(
    out: &mut String,
    pattern: &str,
    info: &NumberFormatInfo,
    symbol: &str,
    mut digits: impl FnMut(&mut String),
) {
    for ch in pattern.chars() {
        match ch {
            '#' => digits(out),
            '-' => out.push_str(info.negative_sign()),
            '$' | '%' => out.push_str(symbol),
            other => out.push(other),
        }
    }
}

fn number_to_string(
    out: &mut String,
    number: &mut Number,
    format: u8,
    max_digits: i32,
    info: &NumberFormatInfo,
) -> Result<(), FormatError> {
    let correctly_rounded = number.floating;
    let mut max_digits = max_digits;
    match format {
        b'C' | b'c' => {
            if max_digits < 0 {
                max_digits = info.currency_decimal_digits();
            }
            round_number(number, i64::from(number.scale) + i64::from(max_digits), correctly_rounded);
            let layout = if number.negative {
                pattern(&NEGATIVE_CURRENCY_FORMATS, info.currency_negative_pattern())
            } else {
                pattern(&POSITIVE_CURRENCY_FORMATS, info.currency_positive_pattern())
            };
            write_pattern(out, layout, info, info.currency_symbol(), |out| {
                format_fixed(
                    out,
                    number,
                    max_digits,
                    Some(info.currency_group_sizes()),
                    info.currency_decimal_separator(),
                    info.currency_group_separator(),
                )
            });
        }
        b'F' | b'f' => {
            if max_digits < 0 {
                max_digits = info.number_decimal_digits();
            }
            round_number(number, i64::from(number.scale) + i64::from(max_digits), correctly_rounded);
            if number.negative {
                out.push_str(info.negative_sign());
            }
            format_fixed(out, number, max_digits, None, info.number_decimal_separator(), "");
        }
        b'N' | b'n' => {
            if max_digits < 0 {
                max_digits = info.number_decimal_digits();
            }
            round_number(number, i64::from(number.scale) + i64::from(max_digits), correctly_rounded);
            let layout =
                if number.negative { pattern(&NEGATIVE_NUMBER_FORMATS, info.number_negative_pattern()) } else { "#" };
            write_pattern(out, layout, info, "", |out| {
                format_fixed(
                    out,
                    number,
                    max_digits,
                    Some(info.number_group_sizes()),
                    info.number_decimal_separator(),
                    info.number_group_separator(),
                )
            });
        }
        b'E' | b'e' => {
            if max_digits < 0 {
                max_digits = 6;
            }
            max_digits = max_digits.saturating_add(1);
            round_number(number, i64::from(max_digits), correctly_rounded);
            if number.negative {
                out.push_str(info.negative_sign());
            }
            format_scientific(out, number, max_digits, format as char, info);
        }
        b'G' | b'g' | b'R' | b'r' => {
            if max_digits < 1 {
                max_digits = number.digits.len() as i32;
            }
            round_number(number, i64::from(max_digits), correctly_rounded);
            if number.negative {
                out.push_str(info.negative_sign());
            }
            let exponent_char = if format.is_ascii_uppercase() { 'E' } else { 'e' };
            format_general(out, number, max_digits, exponent_char, info);
        }
        b'P' | b'p' => {
            if max_digits < 0 {
                max_digits = info.percent_decimal_digits();
            }
            number.scale += 2;
            round_number(number, i64::from(number.scale) + i64::from(max_digits), correctly_rounded);
            let layout = if number.negative {
                pattern(&NEGATIVE_PERCENT_FORMATS, info.percent_negative_pattern())
            } else {
                pattern(&POSITIVE_PERCENT_FORMATS, info.percent_positive_pattern())
            };
            write_pattern(out, layout, info, info.percent_symbol(), |out| {
                format_fixed(
                    out,
                    number,
                    max_digits,
                    Some(info.percent_group_sizes()),
                    info.percent_decimal_separator(),
                    info.percent_group_separator(),
                )
            });
        }
        _ => return Err(FormatError::InvalidFormatSpecifier),
    }
    Ok(())
}

/// The digit counts (from the decimal separator leftwards) after which a
/// group separator is written for `digits` integer digits: the group sizes
/// in order, the last one repeating; a size of zero ends the grouping.
fn group_positions(digits: i32, sizes: &[i32]) -> Vec<i32> {
    let mut positions = Vec::new();
    let Some(&first) = sizes.first() else { return positions };
    let mut index = 0;
    let mut total = first;
    while digits > total {
        if sizes[index] == 0 {
            break;
        }
        positions.push(total);
        if index + 1 < sizes.len() {
            index += 1;
        }
        if sizes[index] == 0 {
            break;
        }
        total += sizes[index];
    }
    positions
}

fn format_fixed(
    out: &mut String,
    number: &Number,
    max_digits: i32,
    group_sizes: Option<&[i32]>,
    decimal_separator: &str,
    group_separator: &str,
) {
    let mut cursor = 0usize;
    let mut dig_pos = number.scale;
    if dig_pos > 0 {
        let positions = group_sizes.map(|sizes| group_positions(dig_pos, sizes)).unwrap_or_default();
        let count = dig_pos as usize;
        for i in 0..count {
            out.push(number.digit(cursor));
            cursor += 1;
            let remaining = (count - i - 1) as i32;
            if remaining > 0 && positions.contains(&remaining) {
                out.push_str(group_separator);
            }
        }
        dig_pos = 0;
    } else {
        out.push('0');
    }

    if max_digits > 0 {
        let mut remaining = max_digits;
        out.push_str(decimal_separator);
        if dig_pos < 0 {
            let zeros = (-dig_pos).min(remaining);
            push_zeros(out, zeros as usize);
            remaining -= zeros;
        }
        while remaining > 0 {
            out.push(number.digit(cursor));
            cursor += 1;
            remaining -= 1;
        }
    }
}

fn format_scientific(out: &mut String, number: &Number, max_digits: i32, exponent_char: char, info: &NumberFormatInfo) {
    out.push(number.digit(0));
    if max_digits != 1 {
        // E0 has no decimal point.
        out.push_str(info.number_decimal_separator());
    }
    for i in 1..max_digits.max(1) as usize {
        out.push(number.digit(i));
    }
    let exponent = if number.is_zero() { 0 } else { number.scale - 1 };
    format_exponent(out, exponent, exponent_char, 3, true, info);
}

fn format_exponent(
    out: &mut String,
    value: i32,
    exponent_char: char,
    min_digits: usize,
    positive_sign: bool,
    info: &NumberFormatInfo,
) {
    out.push(exponent_char);
    if value < 0 {
        out.push_str(info.negative_sign());
    } else if positive_sign {
        out.push_str(info.positive_sign());
    }
    let digits = value.unsigned_abs().to_string();
    push_zeros(out, min_digits.saturating_sub(digits.len()));
    out.push_str(&digits);
}

fn format_general(out: &mut String, number: &Number, max_digits: i32, exponent_char: char, info: &NumberFormatInfo) {
    let mut dig_pos = number.scale;
    let mut scientific = false;
    if dig_pos > max_digits || dig_pos < -3 {
        dig_pos = 1;
        scientific = true;
    }

    let mut cursor = 0usize;
    if dig_pos > 0 {
        while dig_pos > 0 {
            out.push(number.digit(cursor));
            cursor += 1;
            dig_pos -= 1;
        }
    } else {
        out.push('0');
    }

    if cursor < number.digits.len() || dig_pos < 0 {
        out.push_str(info.number_decimal_separator());
        while dig_pos < 0 {
            out.push('0');
            dig_pos += 1;
        }
        while cursor < number.digits.len() {
            out.push(number.digit(cursor));
            cursor += 1;
        }
    }

    if scientific {
        format_exponent(out, number.scale - 1, exponent_char, 2, true, info);
    }
}

// ---------------------------------------------------------------------------
// Custom numeric format strings
// ---------------------------------------------------------------------------

/// Finds the start of a `;` separated section of a custom format string, or
/// 0 if the section does not exist or is empty.
fn find_section(format: &[char], section: i32) -> usize {
    if section == 0 {
        return 0;
    }
    let mut section = section;
    let mut src = 0;
    loop {
        if src >= format.len() {
            return 0;
        }
        let ch = format[src];
        src += 1;
        match ch {
            '\'' | '"' => {
                while src < format.len() && format[src] != '\0' {
                    let c = format[src];
                    src += 1;
                    if c == ch {
                        break;
                    }
                }
            }
            '\\' => {
                if src < format.len() && format[src] != '\0' {
                    src += 1;
                }
            }
            ';' => {
                section -= 1;
                if section != 0 {
                    continue;
                }
                if src < format.len() && format[src] != '\0' && format[src] != ';' {
                    return src;
                }
                return 0;
            }
            '\0' => return 0,
            _ => {}
        }
    }
}

fn number_to_string_format(out: &mut String, number: &mut Number, format: &[char], info: &NumberFormatInfo) {
    let len = format.len();
    let at = |index: usize| format.get(index).copied().unwrap_or('\0');

    let mut section = find_section(format, if number.is_zero() { 2 } else if number.negative { 1 } else { 0 });
    let mut digit_count: i32;
    let mut decimal_pos: i32;
    let mut first_digit: i32;
    let mut last_digit: i32;
    let mut scientific: bool;
    let mut thousand_pos: i32;
    let mut thousand_count: i32 = 0;
    let mut thousand_seps: bool;
    let mut scale_adjust: i32;

    loop {
        digit_count = 0;
        decimal_pos = -1;
        first_digit = i32::MAX;
        last_digit = 0;
        scientific = false;
        thousand_pos = -1;
        thousand_seps = false;
        scale_adjust = 0;
        let mut src = section;

        while src < len {
            let ch = format[src];
            src += 1;
            if ch == '\0' || ch == ';' {
                break;
            }
            match ch {
                '#' => digit_count += 1,
                '0' => {
                    if first_digit == i32::MAX {
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
                PER_MILLE_SYMBOL => scale_adjust += 3,
                '\'' | '"' => {
                    while src < len && format[src] != '\0' {
                        let c = format[src];
                        src += 1;
                        if c == ch {
                            break;
                        }
                    }
                }
                '\\' => {
                    if src < len && format[src] != '\0' {
                        src += 1;
                    }
                }
                'E' | 'e' => {
                    if at(src) == '0' || ((at(src) == '+' || at(src) == '-') && at(src + 1) == '0') {
                        src += 1;
                        while src < len && format[src] == '0' {
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

        if !number.is_zero() {
            number.scale += scale_adjust;
            let pos = if scientific {
                i64::from(digit_count)
            } else {
                i64::from(number.scale) + i64::from(digit_count) - i64::from(decimal_pos)
            };
            round_number(number, pos, false);
            if number.is_zero() {
                let zero_section = find_section(format, 2);
                if zero_section != section {
                    section = zero_section;
                    continue;
                }
            }
        } else {
            if !number.floating {
                // Integers have no negative zero.
                number.negative = false;
            }
            number.scale = 0;
        }
        break;
    }

    let first_digit = if first_digit < decimal_pos { decimal_pos - first_digit } else { 0 };
    let last_digit = if last_digit > decimal_pos { decimal_pos - last_digit } else { 0 };
    let mut dig_pos: i32;
    let mut adjust: i32;
    if scientific {
        dig_pos = decimal_pos;
        adjust = 0;
    } else {
        dig_pos = number.scale.max(decimal_pos);
        adjust = number.scale - decimal_pos;
    }

    // The digit positions after which a group separator is written.
    let mut separator_positions: Vec<i32> = Vec::new();
    if thousand_seps {
        let total_digits = dig_pos + adjust.min(0);
        let num_digits = first_digit.max(total_digits);
        separator_positions = group_positions(num_digits, info.number_group_sizes());
    }

    // A negative number that is (or rounded to) zero only gets its sign if
    // the format produces any text: see the end.
    let start = out.len();
    if number.negative && section == 0 && number.scale != 0 {
        out.push_str(info.negative_sign());
    }

    let mut decimal_written = false;
    let mut cursor = 0usize;
    let mut src = section;
    let digits_len = number.digits.len();

    let mut write_separator = |out: &mut String, dig_pos: i32| {
        if thousand_seps && dig_pos > 1 {
            if let Some(&last) = separator_positions.last() {
                if dig_pos == last + 1 {
                    out.push_str(info.number_group_separator());
                    separator_positions.pop();
                }
            }
        }
    };

    while src < len {
        let ch = format[src];
        src += 1;
        if ch == '\0' || ch == ';' {
            break;
        }

        if adjust > 0 && matches!(ch, '#' | '0' | '.') {
            // The number has more integer digits than the format.
            while adjust > 0 {
                out.push(number.digit(cursor));
                cursor += 1;
                write_separator(out, dig_pos);
                dig_pos -= 1;
                adjust -= 1;
            }
        }

        match ch {
            '#' | '0' => {
                let digit = if adjust < 0 {
                    adjust += 1;
                    if dig_pos <= first_digit { Some('0') } else { None }
                } else if cursor < digits_len {
                    cursor += 1;
                    Some(number.digit(cursor - 1))
                } else if dig_pos > last_digit {
                    Some('0')
                } else {
                    None
                };
                if let Some(digit) = digit {
                    out.push(digit);
                    write_separator(out, dig_pos);
                }
                dig_pos -= 1;
            }
            '.' => {
                // Repeated decimal points are not echoed.
                if dig_pos == 0
                    && !decimal_written
                    && (last_digit < 0 || (decimal_pos < digit_count && cursor < digits_len))
                {
                    out.push_str(info.number_decimal_separator());
                    decimal_written = true;
                }
            }
            PER_MILLE_SYMBOL => out.push_str(info.per_mille_symbol()),
            '%' => out.push_str(info.percent_symbol()),
            ',' => {}
            '\'' | '"' => {
                while src < len && format[src] != '\0' && format[src] != ch {
                    out.push(format[src]);
                    src += 1;
                }
                if src < len && format[src] != '\0' {
                    src += 1;
                }
            }
            '\\' => {
                if src < len && format[src] != '\0' {
                    out.push(format[src]);
                    src += 1;
                }
            }
            'E' | 'e' => {
                if scientific {
                    let mut positive_sign = false;
                    let mut min_digits = 0usize;
                    if at(src) == '0' {
                        // E0 formats like E-0.
                        min_digits += 1;
                    } else if at(src) == '+' && at(src + 1) == '0' {
                        positive_sign = true;
                    } else if at(src) == '-' && at(src + 1) == '0' {
                        // E-0: nothing to do.
                    } else {
                        out.push(ch);
                        continue;
                    }
                    src += 1;
                    while src < len && format[src] == '0' {
                        min_digits += 1;
                        src += 1;
                    }
                    let exponent = if number.is_zero() { 0 } else { number.scale - decimal_pos };
                    format_exponent(out, exponent, ch, min_digits.min(10), positive_sign, info);
                    scientific = false;
                } else {
                    out.push(ch);
                    if src < len {
                        if format[src] == '+' || format[src] == '-' {
                            out.push(format[src]);
                            src += 1;
                        }
                        while src < len && format[src] == '0' {
                            out.push(format[src]);
                            src += 1;
                        }
                    }
                }
            }
            _ => out.push(ch),
        }
    }

    if number.negative && section == 0 && number.scale == 0 && out.len() > start {
        out.insert_str(start, info.negative_sign());
    }
}
