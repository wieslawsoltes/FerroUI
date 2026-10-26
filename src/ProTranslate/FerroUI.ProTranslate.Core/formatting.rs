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
        if !first.is_ascii_alphabetic() || !specifier.as_bytes()[1..].iter().all(u8::is_ascii_digit)
        {
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
                    b'C' => {
                        let profile = profile.expect("currency profile exists");
                        let (pattern, symbol) = if culture.is_invariant() { ("¤#,##0.00;(¤#,##0.00)", "¤") } else { (metadata.4, profile.currency_symbol.as_str()) };
                        Ok(apply_affix(pattern, &number, negative, symbol, metadata.6))
                    },
                    b'P' => Ok(apply_affix(if culture.is_invariant() { "#,##0 %" } else { metadata.5 }, &number, negative, "", metadata.6)),
                    _ => Ok(number),
                }
            }
            b'E' => scientific(decimal, culture, Some(precision.unwrap_or(6)), first as char, 3),
            b'G' | b'R' => {
                if code == b'R' && precision.is_some() { return Err(error("round-trip format does not accept a precision")); }
                if let Some(digits) = precision.filter(|digits| *digits > 0) {
                    let magnitude = decimal.nonzero_magnitude_start();
                    decimal.round(magnitude - digits as i16 + 1); decimal.trim_end();
                    let magnitude = decimal.nonzero_magnitude_start();
                    if magnitude < -4 || magnitude >= digits as i16 { return scientific(decimal, culture, None, if first.is_ascii_lowercase() { 'e' } else { 'E' }, 2); }
                }
                if matches!(self, Self::Number(_)) && precision.is_none_or(|precision| precision == 0) {
                    let magnitude = decimal.nonzero_magnitude_start();
                    if !(-4..17).contains(&magnitude) { return scientific(decimal, culture, None, if first.is_ascii_lowercase() { 'e' } else { 'E' }, 2); }
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
    digits: Option<usize>,
    letter: char,
    exponent_width: usize,
) -> Result<String, TranslationError> {
    let mut exponent = decimal.nonzero_magnitude_start();
    decimal.multiply_pow10(-exponent);
    if let Some(digits) = digits {
        decimal.round(-(digits as i16));
    }
    if decimal.nonzero_magnitude_start() > 0 {
        decimal.multiply_pow10(-1);
        exponent += 1;
    }
    decimal.trim_start();
    if let Some(digits) = digits {
        decimal.pad_end(-(digits as i16));
    } else {
        decimal.trim_end();
    }
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
#[derive(Debug, Clone)]
enum NumericToken {
    Zero,
    Optional,
    Point,
    Comma,
    Percent,
    PerMille,
    Literal(String),
}
fn custom_sections(pattern: &str) -> Result<Vec<Vec<NumericToken>>, TranslationError> {
    let mut sections = vec![Vec::new()];
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        let token = match ch {
            ';' => {
                if sections.len() == 3 {
                    return Err(error("numeric custom format has more than three sections"));
                }
                sections.push(Vec::new());
                continue;
            }
            '\\' => NumericToken::Literal(
                chars
                    .next()
                    .ok_or_else(|| error("trailing custom numeric escape"))?
                    .to_string(),
            ),
            '\'' | '"' => {
                let mut literal = String::new();
                let mut closed = false;
                while let Some(next) = chars.next() {
                    if next == ch {
                        closed = true;
                        break;
                    }
                    if next == '\\' {
                        literal.push(
                            chars
                                .next()
                                .ok_or_else(|| error("trailing quoted numeric escape"))?,
                        );
                    } else {
                        literal.push(next);
                    }
                }
                if !closed {
                    return Err(error("unclosed numeric format quote"));
                }
                NumericToken::Literal(literal)
            }
            '0' => NumericToken::Zero,
            '#' => NumericToken::Optional,
            '.' => NumericToken::Point,
            ',' => NumericToken::Comma,
            '%' => NumericToken::Percent,
            '‰' => NumericToken::PerMille,
            'e' | 'E' if chars.peek().is_some_and(|ch| matches!(ch, '0' | '+' | '-')) => {
                return Err(error(
                    "custom scientific masks are unsupported; use E or G numeric formats",
                ))
            }
            _ => NumericToken::Literal(ch.to_string()),
        };
        sections
            .last_mut()
            .expect("at least one section")
            .push(token);
    }
    Ok(sections)
}
fn format_custom(
    decimal: Decimal,
    pattern: &str,
    culture: &Culture,
) -> Result<String, TranslationError> {
    let sections = custom_sections(pattern)?;
    let negative = decimal.sign() == Sign::Negative && !decimal.is_zero();
    let selected = if decimal.is_zero() && sections.len() == 3 && !sections[2].is_empty() {
        2
    } else if negative && sections.len() >= 2 && !sections[1].is_empty() {
        1
    } else {
        0
    };
    render_custom(
        decimal,
        &sections[selected],
        negative && selected == 0,
        culture,
        if selected != 2 && sections.len() == 3 {
            Some(&sections[2])
        } else {
            None
        },
    )
}
fn render_custom(
    mut decimal: Decimal,
    tokens: &[NumericToken],
    automatic_minus: bool,
    culture: &Culture,
    zero_section: Option<&Vec<NumericToken>>,
) -> Result<String, TranslationError> {
    let positions: Vec<_> = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            matches!(token, NumericToken::Zero | NumericToken::Optional).then_some(index)
        })
        .collect();
    let metadata = locale_metadata(culture);
    let mut number = None;
    let mut numeric_start = usize::MAX;
    let mut numeric_end = 0;
    if !positions.is_empty() {
        let points: Vec<_> = tokens
            .iter()
            .enumerate()
            .filter_map(|(index, token)| matches!(token, NumericToken::Point).then_some(index))
            .collect();
        if points.len() > 1 {
            return Err(error("custom numeric pattern has multiple decimal points"));
        }
        let point = points.first().copied();
        numeric_start = positions[0].min(point.unwrap_or(positions[0]));
        numeric_end = *positions.last().expect("positions nonempty");
        for token in &tokens[numeric_start..=numeric_end] {
            if !matches!(
                token,
                NumericToken::Zero
                    | NumericToken::Optional
                    | NumericToken::Point
                    | NumericToken::Comma
            ) {
                return Err(error("interleaved numeric masks are unsupported; put literals before or after the number"));
            }
        }
        let integer_positions: Vec<_> = positions
            .iter()
            .copied()
            .filter(|index| point.is_none_or(|point| *index < point))
            .collect();
        let fraction_positions: Vec<_> = positions
            .iter()
            .copied()
            .filter(|index| point.is_some_and(|point| *index > point))
            .collect();
        let min_integer = integer_positions
            .iter()
            .position(|index| matches!(tokens[*index], NumericToken::Zero))
            .map_or(0, |index| integer_positions.len() - index);
        let min_fraction = fraction_positions
            .iter()
            .rposition(|index| matches!(tokens[*index], NumericToken::Zero))
            .map_or(0, |index| index + 1);
        if min_integer > 1024 || fraction_positions.len() > 1024 {
            return Err(error("numeric placeholder precision exceeds 1024"));
        }
        let last_integer = integer_positions.last().copied();
        let scaling_commas = tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                matches!(token, NumericToken::Comma)
                    && ((last_integer.is_some_and(|last| *index > last)
                        && point.is_none_or(|point| *index < point))
                        || *index > numeric_end)
            })
            .count();
        let grouping = tokens.iter().enumerate().any(|(index, token)| {
            matches!(token, NumericToken::Comma) && last_integer.is_some_and(|last| index < last)
        });
        if tokens.iter().enumerate().any(|(index, token)| {
            matches!(token, NumericToken::Comma)
                && point.is_some_and(|point| index > point)
                && index < numeric_end
        }) {
            return Err(error(
                "grouping separators cannot occur within fractional digit placeholders",
            ));
        }
        let percent_count = tokens
            .iter()
            .filter(|token| matches!(token, NumericToken::Percent))
            .count();
        let permille_count = tokens
            .iter()
            .filter(|token| matches!(token, NumericToken::PerMille))
            .count();
        let scale =
            (percent_count as i64) * 2 + (permille_count as i64) * 3 - (scaling_commas as i64) * 3;
        let scale = i16::try_from(scale)
            .map_err(|_| error("custom numeric scaling exceeds supported magnitude"))?;
        decimal.multiply_pow10(scale);
        decimal.trim_start();
        decimal.round(-(fraction_positions.len() as i16));
        decimal.trim_end();
        if decimal.is_zero() {
            if let Some(zero) = zero_section.filter(|zero| !zero.is_empty()) {
                return render_custom(decimal, zero, false, culture, None);
            }
        }
        decimal.pad_end(-(min_fraction as i16));
        decimal.pad_start(min_integer as i16);
        decimal.set_sign(Sign::None);
        let mut rendered = render_decimal(&decimal, culture, grouping)?;
        if min_integer == 0 && decimal.nonzero_magnitude_start() <= 0 && decimal.digit_at(0) == 0 {
            let zero = render_decimal(&Decimal::from(0), culture, false)?;
            if let Some(without_zero) = rendered.strip_prefix(&zero) {
                rendered = without_zero.into();
            }
        }
        number = Some(rendered);
    }
    let mut output = String::new();
    if automatic_minus {
        output.push('-');
    }
    for (index, token) in tokens.iter().enumerate() {
        if index == numeric_start {
            if let Some(number) = &number {
                output.push_str(number);
            }
        }
        if index >= numeric_start && index <= numeric_end {
            continue;
        }
        match token {
            NumericToken::Literal(value) => output.push_str(value),
            NumericToken::Percent => output.push_str(metadata.6),
            NumericToken::PerMille => output.push('‰'),
            NumericToken::Zero
            | NumericToken::Optional
            | NumericToken::Point
            | NumericToken::Comma => {}
        }
    }
    Ok(output)
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
