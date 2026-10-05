use super::FlexBasisKind;
use ferroui_base::utilities::span_helpers::{try_parse_double, NumberStyles};
use ferroui_base::utilities::FormatError;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// Specifies the initial size of a flex item.
///
/// The default value is [`FlexBasis::AUTO`].
#[derive(Clone, Copy, Debug, Default)]
pub struct FlexBasis {
    value: f64,
    kind: FlexBasisKind,
}

impl FlexBasis {
    /// A basis that represents the "auto" value, which means the size of the
    /// flex item is determined by its content or other factors.
    pub const AUTO: FlexBasis = FlexBasis { value: 0.0, kind: FlexBasisKind::Auto };

    /// Creates a basis with the given value and kind. The kind determines
    /// how the value affects the size of the flex item.
    ///
    /// Panics if the value is negative, NaN or infinite.
    pub fn new(value: f64, kind: FlexBasisKind) -> Self {
        if !Self::is_valid_value(value) {
            panic!("Invalid basis value: {value}");
        }

        Self { value, kind }
    }

    /// Creates a basis with the given absolute value.
    ///
    /// Panics if the value is negative, NaN or infinite.
    pub fn absolute(value: f64) -> Self {
        Self::new(value, FlexBasisKind::Absolute)
    }

    /// A basis from parts that are known to be valid.
    #[cfg(test)]
    pub(crate) const fn from_parts(value: f64, kind: FlexBasisKind) -> Self {
        Self { value, kind }
    }

    fn is_valid_value(value: f64) -> bool {
        !(value < 0.0 || value.is_nan() || value.is_infinite())
    }

    /// The value of the basis. The meaning of this value depends on the
    /// [`FlexBasisKind`].
    #[inline]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// The kind of the basis. This determines how the value affects the size
    /// of the flex item.
    #[inline]
    pub const fn kind(&self) -> FlexBasisKind {
        self.kind
    }

    /// A basis that represents the "auto" value, which means the size of the
    /// flex item is determined by its content or other factors.
    #[inline]
    pub const fn auto() -> Self {
        Self::AUTO
    }

    /// Whether the basis is set to "auto".
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.kind == FlexBasisKind::Auto
    }

    /// Whether the basis is set to an absolute value.
    #[inline]
    pub fn is_absolute(&self) -> bool {
        self.kind == FlexBasisKind::Absolute
    }

    /// Whether the basis is set to a relative value.
    #[inline]
    pub fn is_relative(&self) -> bool {
        self.kind == FlexBasisKind::Relative
    }

    /// Converts a string flex-basis value to a [`FlexBasis`].
    ///
    /// Valid values are `auto` (in any casing), a number (`100`) or a
    /// percentage (`50%`).
    pub fn parse(str: &str) -> Result<FlexBasis, FormatError> {
        let span = str.trim();
        // As upstream, "auto" is recognised without surrounding white space
        // only and a number is parsed from the untrimmed text (the number
        // style allows white space around it).
        if str.eq_ignore_ascii_case("AUTO") {
            return Ok(Self::AUTO);
        } else if let Some(val) = span
            .strip_suffix('%')
            .and_then(|percentage| try_parse_double(percentage, NumberStyles::ALLOW_DECIMAL_POINT))
        {
            if Self::is_valid_value(val / 100.0) {
                return Ok(Self { value: val / 100.0, kind: FlexBasisKind::Relative });
            }
        } else if let Some(value) = try_parse_double(str, NumberStyles::FLOAT) {
            if Self::is_valid_value(value) {
                return Ok(Self { value, kind: FlexBasisKind::Absolute });
            }
        }

        Err(FormatError::from_string(format!(
            "Value '{str}' is not a valid flex-basis value. Valid values are 'auto', a number (e.g., '100'), or a percentage (e.g., '50%')."
        )))
    }
}

impl PartialEq for FlexBasis {
    fn eq(&self, other: &Self) -> bool {
        (self.is_auto() && other.is_auto()) || (self.value == other.value && self.kind == other.kind)
    }
}

impl Hash for FlexBasis {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Auto bases compare equal whatever their value is.
        if !self.is_auto() {
            // Positive and negative zero compare equal.
            (self.value + 0.0).to_bits().hash(state);
        }
        self.kind.hash(state);
    }
}

impl fmt::Display for FlexBasis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            FlexBasisKind::Auto => f.write_str("Auto"),
            FlexBasisKind::Absolute => write_general_17(f, self.value),
            FlexBasisKind::Relative => {
                write_general_17(f, self.value * 100.0)?;
                f.write_str("%")
            }
        }
    }
}

impl FromStr for FlexBasis {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Writes `value` as the invariant general number format with 17 significant
/// digits does: the correctly rounded 17 digits without trailing zeros, in
/// fixed notation when the decimal exponent is in `-5..17`, in scientific
/// notation (`E+XX`) otherwise.
fn write_general_17(f: &mut fmt::Formatter<'_>, value: f64) -> fmt::Result {
    const PRECISION: i32 = 17;

    if value.is_nan() {
        return f.write_str("NaN");
    }
    if value.is_infinite() {
        return f.write_str(if value < 0.0 { "-Infinity" } else { "Infinity" });
    }

    if value.is_sign_negative() {
        f.write_str("-")?;
    }

    // `d.dddddddddddddddde[-]x`: the 17 correctly rounded significant digits.
    let scientific = format!("{:.16e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);

    let mut digits = [b'0'; PRECISION as usize];
    let mut count = 0;
    for byte in mantissa.bytes().filter(u8::is_ascii_digit).take(PRECISION as usize) {
        digits[count] = byte;
        count += 1;
    }
    while count > 1 && digits[count - 1] == b'0' {
        count -= 1;
    }
    let digits = &digits[..count];
    let write_digits = |f: &mut fmt::Formatter<'_>, digits: &[u8]| {
        digits.iter().try_for_each(|digit| fmt::Write::write_char(f, *digit as char))
    };

    if exponent > -5 && exponent < PRECISION {
        if exponent < 0 {
            f.write_str("0.")?;
            for _ in 0..(-exponent - 1) {
                f.write_str("0")?;
            }
            write_digits(f, digits)
        } else {
            let integer_digits = exponent as usize + 1;
            if digits.len() <= integer_digits {
                write_digits(f, digits)?;
                for _ in digits.len()..integer_digits {
                    f.write_str("0")?;
                }
                Ok(())
            } else {
                write_digits(f, &digits[..integer_digits])?;
                f.write_str(".")?;
                write_digits(f, &digits[integer_digits..])
            }
        }
    } else {
        write_digits(f, &digits[..1])?;
        if digits.len() > 1 {
            f.write_str(".")?;
            write_digits(f, &digits[1..])?;
        }
        write!(f, "E{}{:02}", if exponent < 0 { '-' } else { '+' }, exponent.abs())
    }
}
