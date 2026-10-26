use crate::{region::locale_metadata, Culture, RegionProfile, TranslationError};
use fixed_decimal::{Decimal, FloatPrecision, Sign, SignedRoundingMode, UnsignedRoundingMode};
use icu_decimal::{options::GroupingStrategy, DecimalFormatter};
use std::{collections::BTreeSet, fmt, rc::Rc};
use writeable::{Part as FormattedPart, PartsWrite, Writeable};

/// Extension point for application values (including dates, money, and domain
/// types). It preserves typed formatting without reflection or string coercion.
pub trait LocalizedFormat {
    fn format(&self, format: &str, culture: &Culture) -> Result<String, TranslationError>;
}
#[derive(Clone)]
pub enum FormatArgument {
    String(String),
    Integer8(i8),
    Integer16(i16),
    Integer32(i32),
    Integer(i64),
    Integer128(i128),
    Unsigned(u64),
    Unsigned128(u128),
    Float(f32),
    Number(f64),
    Bool(bool),
    Null,
    /// Exact base-ten value, useful for money and numbers above 2^53.
    Decimal(Decimal),
    Custom(Rc<dyn LocalizedFormat>),
    DateTime(crate::LocalizedDateTime),
    NaiveDateTime(crate::LocalizedNaiveDateTime),
}
impl fmt::Debug for FormatArgument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(v) => v.fmt(f),
            Self::Integer8(v) => v.fmt(f),
            Self::Integer16(v) => v.fmt(f),
            Self::Integer32(v) => v.fmt(f),
            Self::Integer(v) => v.fmt(f),
            Self::Integer128(v) => v.fmt(f),
            Self::Unsigned(v) => v.fmt(f),
            Self::Unsigned128(v) => v.fmt(f),
            Self::Float(v) => v.fmt(f),
            Self::Number(v) => v.fmt(f),
            Self::Bool(v) => v.fmt(f),
            Self::Null => f.write_str("Null"),
            Self::Decimal(v) => v.fmt(f),
            Self::Custom(_) => f.write_str("Custom(..)"),
            Self::DateTime(value) => value.fmt(f),
            Self::NaiveDateTime(value) => value.fmt(f),
        }
    }
}
impl PartialEq for FormatArgument {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Integer8(a), Self::Integer8(b)) => a == b,
            (Self::Integer16(a), Self::Integer16(b)) => a == b,
            (Self::Integer32(a), Self::Integer32(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Integer128(a), Self::Integer128(b)) => a == b,
            (Self::Unsigned(a), Self::Unsigned(b)) => a == b,
            (Self::Unsigned128(a), Self::Unsigned128(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Null, Self::Null) => true,
            (Self::Decimal(a), Self::Decimal(b)) => a == b,
            (Self::DateTime(a), Self::DateTime(b)) => a == b,
            (Self::NaiveDateTime(a), Self::NaiveDateTime(b)) => a == b,
            (Self::Custom(a), Self::Custom(b)) => Rc::ptr_eq(a, b),
            _ => false,
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
        Self::Float(value)
    }
}
impl From<bool> for FormatArgument {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
macro_rules! signed { ($($ty:ty => $variant:ident),*) => { $(impl From<$ty> for FormatArgument { fn from(value: $ty) -> Self { Self::$variant(value) } })* }; }
macro_rules! unsigned { ($($ty:ty),*) => { $(impl From<$ty> for FormatArgument { fn from(value: $ty) -> Self { Self::Unsigned(value.into()) } })* }; }
signed!(i8 => Integer8, i16 => Integer16, i32 => Integer32, i64 => Integer, i128 => Integer128, u128 => Unsigned128);
unsigned!(u8, u16, u32, u64);
impl From<isize> for FormatArgument {
    fn from(value: isize) -> Self {
        #[cfg(target_pointer_width = "64")]
        {
            Self::Integer(value as i64)
        }
        #[cfg(target_pointer_width = "32")]
        {
            Self::Integer32(value as i32)
        }
        #[cfg(target_pointer_width = "16")]
        {
            Self::Integer16(value as i16)
        }
    }
}
impl From<usize> for FormatArgument {
    fn from(value: usize) -> Self {
        Self::Unsigned(value as u64)
    }
}
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
            Self::NaiveDateTime(value) => value.format(specifier, culture),
            _ => self.format_number(specifier, culture),
        }
    }
    fn decimal_value(&self) -> Result<Decimal, TranslationError> {
        match self {
            Self::Integer8(value) => Ok((*value).into()),
            Self::Integer16(value) => Ok((*value).into()),
            Self::Integer32(value) => Ok((*value).into()),
            Self::Integer(value) => Ok((*value).into()),
            Self::Integer128(value) => Ok((*value).into()),
            Self::Unsigned(value) => Ok((*value).into()),
            Self::Unsigned128(value) => Ok((*value).into()),
            Self::Float(value) => value
                .to_string()
                .parse()
                .map_err(|_| error("number is not finite")),
            Self::Number(value) => Decimal::try_from_f64(*value, FloatPrecision::RoundTrip)
                .map_err(|_| error("number is not finite")),
            Self::Decimal(value) => Ok(value.clone()),
            _ => Err(error("value is not numeric")),
        }
    }
    fn floating_value(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Float(value) => Some(f64::from(*value)),
            _ => None,
        }
    }
    fn integer_bits(&self) -> Option<u128> {
        Some(match self {
            Self::Integer8(value) => *value as u8 as u128,
            Self::Integer16(value) => *value as u16 as u128,
            Self::Integer32(value) => *value as u32 as u128,
            Self::Integer(value) => *value as u64 as u128,
            Self::Integer128(value) => *value as u128,
            Self::Unsigned(value) => *value as u128,
            Self::Unsigned128(value) => *value,
            _ => return None,
        })
    }
    fn format_number(
        &self,
        specifier: &str,
        culture: &Culture,
    ) -> Result<String, TranslationError> {
        let floating = self.floating_value();
        if let Some(value) = floating.filter(|value| !value.is_finite()) {
            let symbols = numeric_symbols(culture);
            return Ok(if value.is_nan() {
                symbols.nan.into()
            } else if value.is_sign_negative() {
                format!("{}{}", symbols.minus, symbols.infinity)
            } else {
                symbols.infinity.into()
            });
        }
        let mut decimal = self.decimal_value()?;
        let specifier = specifier.split('\0').next().unwrap_or_default();
        let specifier = if specifier.is_empty() { "G" } else { specifier };
        let first = specifier.as_bytes()[0];
        if !first.is_ascii_alphabetic() || !specifier.as_bytes()[1..].iter().all(u8::is_ascii_digit)
        {
            // The source runtime retains 15/7 significant digits before custom
            // masks, then uses midpoint-away rounding for the mask itself.
            if let Some(value) = floating {
                decimal = exact_float_decimal(value)?;
                let digits = if matches!(self, Self::Float(_)) {
                    7
                } else {
                    15
                };
                decimal.round(decimal.nonzero_magnitude_start() - digits + 1);
                decimal.trim_end();
            }
            return format_custom(decimal, specifier, culture, floating.is_some());
        }
        let mut precision = if specifier.len() == 1 {
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
        if code == b'R' && floating.is_some() {
            precision = None;
        }
        let rounding = if floating.is_some() {
            UnsignedRoundingMode::HalfEven
        } else {
            UnsignedRoundingMode::HalfExpand
        };
        if let Some(value) = floating {
            if matches!(code, b'F' | b'N' | b'C' | b'P' | b'E')
                || (matches!(code, b'G' | b'R') && precision.is_some_and(|digits| digits > 0))
            {
                decimal = exact_float_decimal(value)?;
            }
        }
        if floating.is_none() && decimal.is_zero() {
            decimal.set_sign(Sign::None);
        }
        match code {
            b'X' | b'B' => {
                let number = self.integer_bits().ok_or_else(|| {
                    error("hexadecimal and binary formats require integer arguments")
                })?;
                let width = precision.unwrap_or(0);
                Ok(match code {
                    b'B' => format!("{number:0width$b}"),
                    _ if first.is_ascii_lowercase() => format!("{number:0width$x}"),
                    _ => format!("{number:0width$X}"),
                })
            }
            b'D' => {
                if self.integer_bits().is_none() {
                    return Err(error("decimal integer format requires an integer argument"));
                }
                decimal.pad_start(precision.unwrap_or(0) as i16);
                render_decimal(&decimal, culture, false)
            }
            b'F' | b'N' | b'C' | b'P' => {
                let metadata = locale_metadata(culture);
                let symbols = numeric_symbols(culture);
                let profile = if code == b'C' {
                    Some(RegionProfile::for_culture(culture)?)
                } else {
                    None
                };
                // CLDR 48 decimal patterns permit three fractional places; the
                // invariant runtime format has its explicit two-place default.
                let digits = precision.unwrap_or_else(|| {
                    profile
                        .as_ref()
                        .map_or(if culture.is_invariant() { 2 } else { 3 }, |profile| {
                            profile.currency_decimal_digits as usize
                        })
                });
                if code == b'P' {
                    scale_decimal(&mut decimal, 2)?;
                }
                round_decimal(&mut decimal, -(digits as i32), rounding)?;
                decimal.pad_end(-(digits as i16));
                if floating.is_none() && decimal.is_zero() {
                    decimal.set_sign(Sign::None);
                }
                let negative = decimal.sign() == Sign::Negative;
                if matches!(code, b'C' | b'P') {
                    decimal.set_sign(Sign::None);
                }
                let number = render_decimal(&decimal, culture, code != b'F')?;
                match code {
                    b'C' => {
                        let profile = profile.expect("currency profile exists");
                        let (pattern, symbol) = if culture.is_invariant() {
                            ("¤#,##0.00;(¤#,##0.00)", "¤")
                        } else {
                            (metadata.4, profile.currency_symbol.as_str())
                        };
                        Ok(apply_affix(pattern, &number, negative, symbol, symbols))
                    }
                    b'P' => Ok(apply_affix(
                        if culture.is_invariant() {
                            "#,##0 %"
                        } else {
                            metadata.5
                        },
                        &number,
                        negative,
                        "",
                        symbols,
                    )),
                    _ => Ok(number),
                }
            }
            b'E' => scientific(
                decimal,
                culture,
                Some(precision.unwrap_or(6)),
                first as char,
                3,
                rounding,
            ),
            b'G' | b'R' => {
                if let Some(digits) = precision.filter(|digits| *digits > 0) {
                    let magnitude = i32::from(decimal.nonzero_magnitude_start());
                    round_decimal(&mut decimal, magnitude - digits as i32 + 1, rounding)?;
                    decimal.trim_end();
                    let magnitude = decimal.nonzero_magnitude_start();
                    if magnitude < -4 || magnitude >= digits as i16 {
                        return scientific(
                            decimal,
                            culture,
                            None,
                            if first.is_ascii_lowercase() { 'e' } else { 'E' },
                            2,
                            rounding,
                        );
                    }
                }
                if floating.is_some() && precision.is_none_or(|precision| precision == 0) {
                    let maximum = if matches!(self, Self::Float(_)) {
                        9
                    } else {
                        17
                    };
                    let magnitude = decimal.nonzero_magnitude_start();
                    if !(-4..maximum).contains(&magnitude) {
                        return scientific(
                            decimal,
                            culture,
                            None,
                            if first.is_ascii_lowercase() { 'e' } else { 'E' },
                            2,
                            rounding,
                        );
                    }
                }
                if precision.is_some() {
                    decimal.trim_end();
                }
                render_decimal(&decimal, culture, false)
            }
            _ => Err(error("unsupported standard numeric format")),
        }
    }
}
fn error(message: &str) -> TranslationError {
    TranslationError::Format(message.into())
}
pub(crate) struct NumericSymbols {
    pub nan: &'static str,
    pub infinity: &'static str,
    pub percent: &'static str,
    pub permille: &'static str,
    pub minus: &'static str,
    pub plus: &'static str,
    pub group: &'static str,
    pub primary_group: u8,
    pub secondary_group: u8,
}
fn numeric_symbols(culture: &Culture) -> &'static NumericSymbols {
    const INVARIANT: NumericSymbols = NumericSymbols {
        nan: "NaN",
        infinity: "Infinity",
        percent: "%",
        permille: "‰",
        minus: "-",
        plus: "+",
        group: ",",
        primary_group: 3,
        secondary_group: 3,
    };
    if culture.is_invariant() {
        return &INVARIANT;
    }
    let locale = locale_metadata(culture).0;
    let data = crate::locale_data::NUMERIC_SYMBOLS;
    let start = data.partition_point(|row| row.0 < locale);
    let end = data.partition_point(|row| row.0 <= locale);
    let entries = &data[start..end];
    let requested = culture
        .locale()
        .extensions
        .unicode
        .keywords
        .get(&icu_locale::extensions::unicode::key!("nu"))
        .map(ToString::to_string);
    entries
        .iter()
        .find(|row| requested.as_deref() == Some(row.1))
        .or_else(|| entries.iter().find(|row| row.2))
        .map_or(&INVARIANT, |row| &row.3)
}
fn exact_float_decimal(value: f64) -> Result<Decimal, TranslationError> {
    let bits = value.to_bits();
    let biased_exponent = ((bits >> 52) & 0x7ff) as i32;
    let significand =
        (bits & ((1u64 << 52) - 1)) | if biased_exponent == 0 { 0 } else { 1u64 << 52 };
    let exponent = if biased_exponent == 0 {
        -1074
    } else {
        biased_exponent - 1075
    };
    let precision = if significand == 0 {
        0
    } else {
        (-exponent - significand.trailing_zeros() as i32).max(0) as usize
    };
    let mut decimal: Decimal = format!("{value:.precision$}")
        .parse()
        .map_err(|_| error("floating-point magnitude exceeds the decimal formatter"))?;
    decimal.trim_end();
    Ok(decimal)
}
fn render_decimal(
    value: &Decimal,
    culture: &Culture,
    grouping: bool,
) -> Result<String, TranslationError> {
    let formatter =
        DecimalFormatter::try_new(culture.locale().into(), GroupingStrategy::Never.into())
            .map_err(|error| TranslationError::Format(error.to_string()))?;
    if !grouping {
        return Ok(formatter.format_to_string(value));
    }
    // ICU's Always option still honors CLDR minimumGroupingDigits, whereas
    // source N/C/P and custom comma masks group every eligible integer.
    let mut writer = GroupedDecimalWriter {
        output: String::new(),
        integer: false,
        remaining: usize::from(*value.magnitude_range().end() as u16) + 1,
        symbols: numeric_symbols(culture),
    };
    formatter
        .format(value)
        .write_to_parts(&mut writer)
        .map_err(|error| TranslationError::Format(error.to_string()))?;
    Ok(writer.output)
}
struct GroupedDecimalWriter {
    output: String,
    integer: bool,
    remaining: usize,
    symbols: &'static NumericSymbols,
}
impl fmt::Write for GroupedDecimalWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if !self.integer {
            self.output.push_str(value);
            return Ok(());
        }
        for digit in value.chars() {
            self.output.push(digit);
            self.remaining -= 1;
            let primary = usize::from(self.symbols.primary_group);
            let secondary = usize::from(self.symbols.secondary_group);
            if primary != 0
                && secondary != 0
                && self.remaining >= primary
                && (self.remaining - primary).is_multiple_of(secondary)
            {
                self.output.push_str(self.symbols.group);
            }
        }
        Ok(())
    }
}
impl PartsWrite for GroupedDecimalWriter {
    type SubPartsWrite = Self;
    fn with_part(
        &mut self,
        part: FormattedPart,
        mut write: impl FnMut(&mut Self) -> fmt::Result,
    ) -> fmt::Result {
        let integer = self.integer;
        self.integer |= part == icu_decimal::parts::INTEGER;
        let result = write(self);
        self.integer = integer;
        result
    }
}
fn scientific(
    mut decimal: Decimal,
    culture: &Culture,
    digits: Option<usize>,
    letter: char,
    exponent_width: usize,
    rounding: UnsignedRoundingMode,
) -> Result<String, TranslationError> {
    let mut exponent = i32::from(decimal.nonzero_magnitude_start());
    scale_decimal(&mut decimal, -exponent)?;
    if let Some(digits) = digits {
        round_decimal(&mut decimal, -(digits as i32), rounding)?;
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
    let symbols = numeric_symbols(culture);
    let mut magnitude = Decimal::from(exponent.unsigned_abs());
    magnitude.pad_start(exponent_width as i16);
    Ok(format!(
        "{}{letter}{}{}",
        render_decimal(&decimal, culture, false)?,
        if exponent < 0 {
            symbols.minus
        } else {
            symbols.plus
        },
        render_decimal(&magnitude, culture, false)?
    ))
}
fn apply_affix(
    pattern: &str,
    number: &str,
    negative: bool,
    currency: &str,
    symbols: &NumericSymbols,
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
        output.push_str(symbols.minus);
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
            output.push_str(symbols.percent);
        } else if ch == '-' {
            output.push_str(symbols.minus);
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
    Exponent {
        letter: char,
        sign: Option<char>,
        width: usize,
    },
    Literal(String),
}
fn custom_sections(pattern: &str) -> Result<Vec<Vec<NumericToken>>, TranslationError> {
    if pattern.len() > 1_000_000 {
        return Err(error("custom numeric format exceeds 1000000 bytes"));
    }
    let mut sections = vec![Vec::new()];
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        let token = match ch {
            ';' => {
                if sections.len() == 3 {
                    break;
                }
                sections.push(Vec::new());
                continue;
            }
            '\\' => match chars.next() {
                Some(next) => NumericToken::Literal(next.to_string()),
                None => break,
            },
            '\'' | '"' => {
                let mut literal = String::new();
                for next in chars.by_ref() {
                    if next == ch {
                        break;
                    }
                    literal.push(next);
                }
                NumericToken::Literal(literal)
            }
            '0' => NumericToken::Zero,
            '#' => NumericToken::Optional,
            '.' => NumericToken::Point,
            ',' => NumericToken::Comma,
            '%' => NumericToken::Percent,
            '‰' => NumericToken::PerMille,
            'e' | 'E' => {
                let mut lookahead = chars.clone();
                let sign = if lookahead.peek().is_some_and(|ch| matches!(ch, '+' | '-')) {
                    lookahead.next()
                } else {
                    None
                };
                if lookahead.peek() == Some(&'0') {
                    if sign.is_some() {
                        chars.next();
                    }
                    let mut width = 0;
                    while chars.peek() == Some(&'0') {
                        chars.next();
                        width += 1;
                    }
                    NumericToken::Exponent {
                        letter: ch,
                        sign,
                        width,
                    }
                } else {
                    NumericToken::Literal(ch.to_string())
                }
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
    mut decimal: Decimal,
    pattern: &str,
    culture: &Culture,
    floating: bool,
) -> Result<String, TranslationError> {
    let sections = custom_sections(pattern)?;
    let zero_section = if sections.len() == 3 && !sections[2].is_empty() {
        2
    } else {
        0
    };
    let negative = decimal.sign() == Sign::Negative;
    let selected = if decimal.is_zero() {
        zero_section
    } else if negative && sections.len() >= 2 && !sections[1].is_empty() {
        1
    } else {
        0
    };
    if !floating && decimal.is_zero() {
        decimal.set_sign(Sign::None);
    }
    render_custom(
        decimal,
        &sections,
        selected,
        zero_section,
        culture,
        floating,
    )
}
fn render_custom(
    mut decimal: Decimal,
    sections: &[Vec<NumericToken>],
    selected: usize,
    zero_section: usize,
    culture: &Culture,
    floating: bool,
) -> Result<String, TranslationError> {
    let tokens = &sections[selected];
    let mut positions = 0;
    let mut decimal_position = None;
    let mut first_required = None;
    let mut last_required = None;
    let mut comma_groups: Vec<(usize, usize)> = Vec::new();
    let mut scale = 0i32;
    let mut scientific = false;
    for token in tokens {
        match token {
            NumericToken::Zero | NumericToken::Optional => {
                if matches!(token, NumericToken::Zero) {
                    first_required.get_or_insert(positions);
                    last_required = Some(positions);
                }
                positions += 1;
            }
            NumericToken::Point => {
                decimal_position.get_or_insert(positions);
            }
            NumericToken::Comma if decimal_position.is_none() && positions > 0 => {
                if let Some((position, count)) = comma_groups
                    .last_mut()
                    .filter(|(position, _)| *position == positions)
                {
                    let _ = position;
                    *count += 1;
                } else {
                    comma_groups.push((positions, 1));
                }
            }
            NumericToken::Percent => scale += 2,
            NumericToken::PerMille => scale += 3,
            NumericToken::Exponent { .. } => scientific = true,
            _ => {}
        }
    }
    let integers = decimal_position.unwrap_or(positions);
    let fractions = positions - integers;
    let minimum_integer = if scientific {
        integers
    } else {
        first_required
            .filter(|index| *index < integers)
            .map_or(0, |index| integers - index)
    };
    let minimum_fraction = last_required
        .filter(|index| *index >= integers)
        .map_or(0, |index| index - integers + 1);
    if positions > 1024 {
        return Err(error("numeric placeholder precision exceeds 1024"));
    }
    let scaling_commas = comma_groups
        .last()
        .filter(|(position, _)| *position == integers)
        .map_or(0, |(_, count)| *count);
    scale -= (scaling_commas * 3) as i32;
    let grouping = comma_groups
        .iter()
        .any(|(position, _)| *position < integers);
    scale_decimal(&mut decimal, scale)?;
    let position = if scientific {
        i32::from(decimal.nonzero_magnitude_start()) - positions as i32 + 1
    } else {
        -(fractions as i32)
    };
    round_decimal(&mut decimal, position, UnsignedRoundingMode::HalfExpand)?;
    decimal.trim_end();
    if decimal.is_zero() {
        if !floating {
            decimal.set_sign(Sign::None);
        }
        if selected != zero_section {
            return render_custom(
                decimal,
                sections,
                zero_section,
                zero_section,
                culture,
                floating,
            );
        }
    }
    let negative = decimal.sign() == Sign::Negative;
    decimal.set_sign(Sign::None);
    let exponent = if scientific && !decimal.is_zero() {
        i32::from(decimal.nonzero_magnitude_start()) + 1 - integers as i32
    } else {
        0
    };
    if scientific {
        scale_decimal(&mut decimal, -exponent)?;
    }
    let integer_count = if decimal.is_zero() {
        minimum_integer
    } else {
        (i32::from(decimal.nonzero_magnitude_start()) + 1)
            .max(minimum_integer as i32)
            .max(0) as usize
    };
    let fraction_count = if decimal.is_zero() {
        minimum_fraction
    } else {
        (-i32::from(decimal.nonzero_magnitude_end()))
            .max(minimum_fraction as i32)
            .max(0) as usize
    }
    .min(fractions);
    let formatter =
        DecimalFormatter::try_new(culture.locale().into(), GroupingStrategy::Never.into())
            .map_err(|error| TranslationError::Format(error.to_string()))?;
    let digits: Vec<String> = (0u8..10)
        .map(|digit| formatter.format_to_string(&Decimal::from(digit)))
        .collect();
    let digit_characters: Vec<char> = digits
        .iter()
        .filter_map(|digit| digit.chars().next())
        .collect();
    let mut integral = decimal.clone();
    integral.trunc(0);
    integral.trim_end();
    integral.pad_start(minimum_integer as i16);
    let mut integer_cells: Vec<String> = Vec::new();
    if integer_count > 0 {
        for character in render_decimal(&integral, culture, grouping)?.chars() {
            if digit_characters.contains(&character) {
                integer_cells.push(character.to_string());
            } else if let Some(previous) = integer_cells.last_mut() {
                previous.push(character);
            }
        }
    }
    let decimal_separator =
        formatter.format_to_string(&"1.1".parse::<Decimal>().expect("literal decimal"));
    let decimal_separator = decimal_separator
        .strip_prefix(&digits[1])
        .and_then(|value| value.strip_suffix(&digits[1]))
        .unwrap_or(".");
    let symbols = numeric_symbols(culture);
    let adjustment = integer_cells.len() as isize - integers as isize;
    let extra = adjustment.max(0) as usize;
    let mut numeric_index = 0usize;
    let mut extra_written = false;
    let mut point_written = false;
    let mut exponent_written = false;
    let mut output = String::new();
    for token in tokens {
        if !extra_written
            && matches!(
                token,
                NumericToken::Zero | NumericToken::Optional | NumericToken::Point
            )
        {
            for digit in &integer_cells[..extra] {
                output.push_str(digit);
            }
            extra_written = true;
        }
        match token {
            NumericToken::Zero | NumericToken::Optional => {
                if numeric_index < integers {
                    let index = numeric_index as isize + adjustment;
                    if index >= 0 {
                        output.push_str(&integer_cells[index as usize]);
                    }
                } else {
                    let index = numeric_index - integers;
                    if index < fraction_count {
                        output.push_str(&digits[decimal.digit_at(-(index as i16) - 1) as usize]);
                    }
                }
                numeric_index += 1;
            }
            NumericToken::Point => {
                if !point_written && numeric_index == integers && fraction_count > 0 {
                    output.push_str(decimal_separator);
                    point_written = true;
                }
            }
            NumericToken::Percent => output.push_str(symbols.percent),
            NumericToken::PerMille => output.push_str(symbols.permille),
            NumericToken::Comma => {}
            NumericToken::Literal(value) => output.push_str(value),
            NumericToken::Exponent {
                letter,
                sign,
                width,
            } => {
                output.push(*letter);
                if exponent_written {
                    if let Some(sign) = sign {
                        output.push(*sign);
                    }
                    output.extend(std::iter::repeat_n('0', *width));
                } else {
                    if exponent < 0 {
                        output.push_str(symbols.minus);
                    } else if *sign == Some('+') {
                        output.push_str(symbols.plus);
                    }
                    let mut value = Decimal::from(exponent.unsigned_abs());
                    value.pad_start((*width).min(10) as i16);
                    output.push_str(&formatter.format_to_string(&value));
                    exponent_written = true;
                }
            }
        }
    }
    if negative && selected == 0 && (!decimal.is_zero() || !output.is_empty()) {
        output.insert_str(0, symbols.minus);
    }
    Ok(output)
}
fn round_decimal(
    decimal: &mut Decimal,
    position: i32,
    rounding: UnsignedRoundingMode,
) -> Result<(), TranslationError> {
    // A precision beyond the stored fractional range cannot change the value.
    let position = i16::try_from(position.max(i32::from(i16::MIN)))
        .map_err(|_| error("numeric rounding exceeds supported magnitude"))?;
    let can_become_zero = decimal.is_zero() || position > decimal.nonzero_magnitude_start();
    decimal.round_with_mode(position, SignedRoundingMode::Unsigned(rounding));
    if !can_become_zero && decimal.is_zero() {
        // fixed_decimal represents a carry beyond its magnitude range as zero;
        // expose that boundary as an error instead of publishing a wrong value.
        return Err(error("numeric rounding exceeds supported magnitude"));
    }
    Ok(())
}
fn scale_decimal(decimal: &mut Decimal, mut scale: i32) -> Result<(), TranslationError> {
    if !decimal.is_zero()
        && (i32::from(decimal.nonzero_magnitude_start()) + scale > i32::from(i16::MAX)
            || i32::from(decimal.nonzero_magnitude_end()) + scale < i32::from(i16::MIN))
    {
        return Err(error("numeric scaling exceeds supported magnitude"));
    }
    decimal.trim_start();
    decimal.trim_end();
    while scale != 0 {
        let step = scale.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        decimal.multiply_pow10(step);
        decimal.trim_start();
        decimal.trim_end();
        scale -= i32::from(step);
    }
    Ok(())
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
