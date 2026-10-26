use crate::{region::locale_metadata, Culture, RegionProfile, TranslationError};
use fixed_decimal::{Decimal, FloatPrecision, Sign};
use icu_decimal::{options::GroupingStrategy, DecimalFormatter};
use std::{collections::BTreeSet, fmt, rc::Rc};

/// Extension point for application values (including dates, money, and domain
/// types). It preserves typed formatting without reflection or string coercion.
pub trait LocalizedFormat {
    fn format(&self, format: &str, culture: &Culture) -> Result<String, TranslationError>;
}
#[derive(Clone)]
pub enum FormatArgument {
    String(String),
    Integer(i64),
    Unsigned(u64),
    Number(f64),
    Bool(bool),
    Null,
    /// Exact base-ten value, useful for money and numbers above 2^53.
    Decimal(Decimal),
    Custom(Rc<dyn LocalizedFormat>),
    DateTime(crate::LocalizedDateTime),
}
impl fmt::Debug for FormatArgument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(v) => v.fmt(f),
            Self::Integer(v) => v.fmt(f),
            Self::Unsigned(v) => v.fmt(f),
            Self::Number(v) => v.fmt(f),
            Self::Bool(v) => v.fmt(f),
            Self::Null => f.write_str("Null"),
            Self::Decimal(v) => v.fmt(f),
            Self::Custom(_) => f.write_str("Custom(..)"),
            Self::DateTime(value) => value.fmt(f),
        }
    }
}
impl From<String> for FormatArgument {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&str> for FormatArgument {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}
impl From<f64> for FormatArgument {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}
impl From<f32> for FormatArgument {
    fn from(value: f32) -> Self {
        Self::Number(value.into())
    }
}
impl From<bool> for FormatArgument {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
macro_rules! signed { ($($ty:ty),*) => { $(impl From<$ty> for FormatArgument { fn from(value: $ty) -> Self { Self::Integer(value.into()) } })* }; }
macro_rules! unsigned { ($($ty:ty),*) => { $(impl From<$ty> for FormatArgument { fn from(value: $ty) -> Self { Self::Unsigned(value.into()) } })* }; }
signed!(i8, i16, i32, i64);
unsigned!(u8, u16, u32, u64);
impl From<Decimal> for FormatArgument {
    fn from(value: Decimal) -> Self {
        Self::Decimal(value)
    }
}
impl FormatArgument {
    pub fn decimal(value: &str) -> Result<Self, TranslationError> {
        value
            .parse()
            .map(Self::Decimal)
            .map_err(|_| error("invalid decimal value"))
    }
    pub fn format(&self, specifier: &str, culture: &Culture) -> Result<String, TranslationError> {
        match self {
            Self::String(value) => Ok(value.clone()),
            Self::Null => Ok(String::new()),
            Self::Bool(value) => Ok(if *value { "True" } else { "False" }.into()),
            Self::Custom(value) => value.format(specifier, culture),
            Self::DateTime(value) => value.format(specifier, culture),
            _ => self.format_number(specifier, culture),
        }
    }
    fn decimal_value(&self) -> Result<Decimal, TranslationError> {
        match self {
            Self::Integer(value) => Ok((*value).into()),
            Self::Unsigned(value) => Ok((*value).into()),
            Self::Number(value) => Decimal::try_from_f64(*value, FloatPrecision::RoundTrip)
                .map_err(|_| error("number is not finite")),
            Self::Decimal(value) => Ok(value.clone()),
            _ => Err(error("value is not numeric")),
        }
    }
    fn format_number(
        &self,
        specifier: &str,
        culture: &Culture,
    ) -> Result<String, TranslationError> {
        if let Self::Number(value) = self {
            if !value.is_finite() {
                return Ok(if value.is_nan() {
                    "NaN"
                } else if value.is_sign_negative() {
                    "-∞"
                } else {
                    "∞"
                }
                .into());
            }
        }
        let mut decimal = self.decimal_value()?;
        let specifier = if specifier.is_empty() { "G" } else { specifier };
        let first = specifier.as_bytes()[0];
        if !first.is_ascii_alphabetic() {
            return format_custom(decimal, specifier, culture);
        }
        if !specifier.as_bytes()[1..].iter().all(u8::is_ascii_digit) {
            return Err(error("invalid numeric precision"));
        }
        let precision = if specifier.len() == 1 {
            None
        } else {
            Some(
                specifier[1..]
                    .parse::<usize>()
                    .map_err(|_| error("precision is too large"))?,
            )
        };
        if precision.is_some_and(|precision| precision > 1024) {
            return Err(error("precision must not exceed 1024"));
        }
        let code = first.to_ascii_uppercase();
        match code {
            b'X' | b'B' => {
                let number = match self { Self::Integer(value) => *value as u64, Self::Unsigned(value) => *value, _ => return Err(error("hexadecimal and binary formats require integer arguments")) };
                let width = precision.unwrap_or(0);
                Ok(match code { b'B' => format!("{number:0width$b}"), _ if first.is_ascii_lowercase() => format!("{number:0width$x}"), _ => format!("{number:0width$X}") })
            }
            b'D' => {
                if !matches!(self, Self::Integer(_) | Self::Unsigned(_)) { return Err(error("decimal integer format requires an integer argument")); }
                decimal.pad_start(precision.unwrap_or(0) as i16); render_decimal(&decimal, culture, false)
            }
            b'F' | b'N' | b'C' | b'P' => {
                let metadata = locale_metadata(culture);
                let profile = if code == b'C' { Some(RegionProfile::for_culture(culture)?) } else { None };
                let digits = precision.unwrap_or_else(|| profile.as_ref().map_or(2, |profile| profile.currency_decimal_digits as usize));
                if code == b'P' { decimal.multiply_pow10(2); decimal.trim_start(); }
                decimal.round(-(digits as i16)); decimal.pad_end(-(digits as i16));
                let negative = decimal.sign() == Sign::Negative;
                if matches!(code, b'C' | b'P') { decimal.set_sign(Sign::None); }
                let number = render_decimal(&decimal, culture, code != b'F')?;
                match code {
                    b'C' => Ok(apply_affix(metadata.4, &number, negative, &profile.expect("currency profile exists").currency_symbol, metadata.6)),
                    b'P' => Ok(apply_affix(metadata.5, &number, negative, "", metadata.6)),
                    _ => Ok(number),
                }
            }
            b'E' => scientific(decimal, culture, precision.unwrap_or(6), first as char, 3),
            b'G' | b'R' => {
                if code == b'R' && precision.is_some() { return Err(error("round-trip format does not accept a precision")); }
                if let Some(digits) = precision.filter(|digits| *digits > 0) {
                    let magnitude = decimal.nonzero_magnitude_start();
                    decimal.round(magnitude - digits as i16 + 1); decimal.trim_end();
                    let magnitude = decimal.nonzero_magnitude_start();
                    if magnitude < -4 || magnitude >= digits as i16 { return scientific(decimal, culture, digits - 1, if first.is_ascii_lowercase() { 'e' } else { 'E' }, 2); }
                }
                render_decimal(&decimal, culture, false)
            }
            _ => Err(error("unsupported numeric format; use a LocalizedFormat implementation for application-specific values")),
        }
    }
}
fn error(message: &str) -> TranslationError {
    TranslationError::Format(message.into())
}
fn render_decimal(
    value: &Decimal,
    culture: &Culture,
    grouping: bool,
) -> Result<String, TranslationError> {
    let formatter = DecimalFormatter::try_new(
        culture.locale().into(),
        if grouping {
            GroupingStrategy::Always
        } else {
            GroupingStrategy::Never
        }
        .into(),
    )
    .map_err(|error| TranslationError::Format(error.to_string()))?;
    Ok(formatter.format_to_string(value))
}
fn scientific(
    mut decimal: Decimal,
    culture: &Culture,
    digits: usize,
    letter: char,
    exponent_width: usize,
) -> Result<String, TranslationError> {
    let mut exponent = decimal.nonzero_magnitude_start();
    decimal.multiply_pow10(-exponent);
    decimal.round(-(digits as i16));
    if decimal.nonzero_magnitude_start() > 0 {
        decimal.multiply_pow10(-1);
        exponent += 1;
    }
    decimal.pad_end(-(digits as i16));
    Ok(format!(
        "{}{letter}{}{:0width$}",
        render_decimal(&decimal, culture, false)?,
        if exponent < 0 { '-' } else { '+' },
        exponent.unsigned_abs(),
        width = exponent_width
    ))
}
fn apply_affix(
    pattern: &str,
    number: &str,
    negative: bool,
    currency: &str,
    percent: &str,
) -> String {
    let mut parts = pattern.split(';');
    let positive = parts.next().unwrap_or("0");
    let negative_pattern = parts.next();
    let chosen = if negative {
        negative_pattern.unwrap_or(positive)
    } else {
        positive
    };
    let mut output = String::new();
    if negative && negative_pattern.is_none() {
        output.push('-');
    }
    let mut number_written = false;
    let mut quoted = false;
    let mut chars = chosen.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\'' {
            if chars.peek() == Some(&'\'') {
                output.push('\'');
                chars.next();
            } else {
                quoted = !quoted;
            }
        } else if quoted {
            output.push(ch);
        } else if matches!(ch, '#' | '0' | ',' | '.') {
            if !number_written {
                output.push_str(number);
                number_written = true;
            }
        } else if ch == '¤' {
            output.push_str(currency);
        } else if ch == '%' {
            output.push_str(percent);
        } else {
            output.push(ch);
        }
    }
    if !number_written {
        output.push_str(number);
    }
    output
}
fn format_custom(
    mut decimal: Decimal,
    pattern: &str,
    culture: &Culture,
) -> Result<String, TranslationError> {
    // Common .NET fixed numeric patterns are supported. Scientific custom
    // patterns and arbitrary format types use the typed LocalizedFormat hook.
    let parts: Vec<_> = pattern.split(';').collect();
    if parts.len() > 3 {
        return Err(error("numeric custom format has more than three sections"));
    }
    let negative = decimal.sign() == Sign::Negative;
    let zero = decimal.is_zero();
    let section = if zero && parts.len() == 3 {
        parts[2]
    } else if negative && parts.len() >= 2 {
        parts[1]
    } else {
        parts[0]
    };
    let first = section
        .find(['0', '#'])
        .ok_or_else(|| error("custom numeric format requires digit placeholders"))?;
    let last = section.rfind(['0', '#']).expect("first placeholder exists");
    let numeric = &section[first..=last];
    if !numeric
        .chars()
        .all(|ch| matches!(ch, '0' | '#' | ',' | '.'))
        || numeric.matches('.').count() > 1
    {
        return Err(error("unsupported custom numeric pattern"));
    }
    let (integer, fraction) = numeric.split_once('.').unwrap_or((numeric, ""));
    if fraction.contains(',') || fraction.len() > 1024 {
        return Err(error("invalid custom numeric fraction"));
    }
    let min_digits = fraction.chars().filter(|ch| *ch == '0').count();
    if section.contains('%') {
        decimal.multiply_pow10(2);
    }
    if section.contains('‰') {
        decimal.multiply_pow10(3);
    }
    decimal.trim_start();
    decimal.round(-(fraction.len() as i16));
    decimal.trim_end();
    decimal.pad_end(-(min_digits as i16));
    decimal.pad_start(integer.chars().filter(|ch| *ch == '0').count() as i16);
    decimal.set_sign(Sign::None);
    let number = render_decimal(&decimal, culture, integer.contains(','))?;
    let combined = if negative && parts.len() >= 2 {
        format!("{};{}", parts[0], section)
    } else {
        section.into()
    };
    Ok(apply_affix(
        &combined,
        &number,
        negative && !zero,
        "",
        locale_metadata(culture).6,
    ))
}

#[derive(Debug)]
enum Part<'a> {
    Literal(&'a str),
    Escaped(char),
    Argument {
        index: usize,
        alignment: i32,
        format: &'a str,
    },
}
fn parse_format(value: &str) -> Result<Vec<Part<'_>>, TranslationError> {
    let bytes = value.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        if !matches!(bytes[index], b'{' | b'}') {
            index += 1;
            continue;
        }
        if start < index {
            parts.push(Part::Literal(&value[start..index]));
        }
        let brace = bytes[index];
        if bytes.get(index + 1) == Some(&brace) {
            parts.push(Part::Escaped(brace as char));
            index += 2;
            start = index;
            continue;
        }
        if brace == b'}' {
            return Err(error("unexpected closing brace"));
        }
        index += 1;
        let number_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if number_start == index {
            return Err(error("placeholder requires a nonnegative argument index"));
        }
        let argument = value[number_start..index]
            .parse::<usize>()
            .map_err(|_| error("argument index overflows usize"))?;
        if argument > 1_000_000 {
            return Err(error("argument index exceeds 1000000"));
        }
        while bytes.get(index) == Some(&b' ') {
            index += 1;
        }
        let mut alignment = 0;
        if bytes.get(index) == Some(&b',') {
            index += 1;
            while bytes.get(index) == Some(&b' ') {
                index += 1;
            }
            let align_start = index;
            if bytes.get(index) == Some(&b'-') {
                index += 1;
            }
            let digits_start = index;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
            if digits_start == index {
                return Err(error("alignment requires an integer"));
            }
            alignment = value[align_start..index]
                .parse::<i32>()
                .map_err(|_| error("alignment overflows i32"))?;
            if alignment.unsigned_abs() > 1_000_000 {
                return Err(error("alignment exceeds 1000000"));
            }
            while bytes.get(index) == Some(&b' ') {
                index += 1;
            }
        }
        let mut format = "";
        if bytes.get(index) == Some(&b':') {
            index += 1;
            let format_start = index;
            while index < bytes.len() && bytes[index] != b'}' {
                if bytes[index] == b'{' {
                    return Err(error("nested opening brace in format specifier"));
                }
                index += 1;
            }
            format = &value[format_start..index];
        }
        if bytes.get(index) != Some(&b'}') {
            return Err(error("unclosed or malformed placeholder"));
        }
        index += 1;
        parts.push(Part::Argument {
            index: argument,
            alignment,
            format,
        });
        start = index;
    }
    if start < value.len() {
        parts.push(Part::Literal(&value[start..]));
    }
    Ok(parts)
}
/// Validate the composite grammar and return sorted, distinct argument indexes.
pub fn placeholder_indexes(value: &str) -> Result<Vec<usize>, TranslationError> {
    Ok(parse_format(value)?
        .iter()
        .filter_map(|part| {
            if let Part::Argument { index, .. } = part {
                Some(*index)
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}
pub fn required_argument_count(value: &str) -> Result<usize, TranslationError> {
    Ok(placeholder_indexes(value)?
        .last()
        .map_or(0, |index| index + 1))
}
/// Composite formatting with positional arguments, alignment and escaped braces.
/// Alignment counts UTF-16 code units to preserve the source format contract.
pub fn format_composite(
    value: &str,
    arguments: &[FormatArgument],
    culture: &Culture,
) -> Result<String, TranslationError> {
    let parts = parse_format(value)?;
    let mut output = String::with_capacity(value.len());
    for part in parts {
        match part {
            Part::Literal(value) => output.push_str(value),
            Part::Escaped(brace) => output.push(brace),
            Part::Argument {
                index,
                alignment,
                format,
            } => {
                let argument = arguments
                    .get(index)
                    .ok_or_else(|| error("placeholder index exceeds supplied argument count"))?;
                let formatted = argument.format(format, culture)?;
                let spaces = (alignment.unsigned_abs() as usize)
                    .saturating_sub(formatted.encode_utf16().count());
                if alignment > 0 {
                    output.extend(std::iter::repeat_n(' ', spaces));
                }
                output.push_str(&formatted);
                if alignment < 0 {
                    output.extend(std::iter::repeat_n(' ', spaces));
                }
            }
        }
        if output.len() > 16 * 1024 * 1024 {
            return Err(error("formatted output exceeds 16 MiB"));
        }
    }
    Ok(output)
}
