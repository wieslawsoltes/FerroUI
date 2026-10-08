//! A counterpart of .NET's `System.Decimal`: an exact base-10 number with a
//! 96-bit integer mantissa, a sign and a scale (the number of digits after
//! the decimal point) of 0 to 28. The arithmetic is that of the
//! `rust_decimal` crate; this type gives it the members, the text forms and
//! the failure behaviour of the managed type.
//!
//! Only the members the controls use exist: construction and conversion
//! from integers, addition, subtraction, division, comparison, and the
//! formatting and parsing of [`number_format`](super::number_format).
//!
//! The scale is part of the value's representation, as in .NET: `1.10` and
//! `1.1` are equal, hash alike and order alike, but print as `1.10` and
//! `1.1`. Sums and differences take the larger scale. Results outside the
//! value range panic (the overflow exception of the managed runtime); the
//! `checked_*` forms return `None` instead.

use super::number_format::{self, NumberParseError};
use super::{NumberFormatInfo, NumberStyles};
use crate::data::converters::composite_format::FormatError;
use crate::data::core::ValueTypes;
use std::fmt;
use std::ops::{Add, AddAssign, Div, Neg, Sub, SubAssign};
use std::str::FromStr;

/// The largest mantissa: 2^96 - 1.
const MAX_MANTISSA: u128 = (1u128 << 96) - 1;

/// The largest scale.
const MAX_SCALE: u8 = 28;

const OVERFLOW: &str = "Value was either too large or too small for a Decimal.";

/// The powers of ten from 10^0 to 10^28 as doubles.
const DOUBLE_POWERS_10: [f64; 29] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19,
    1e20, 1e21, 1e22, 1e23, 1e24, 1e25, 1e26, 1e27, 1e28,
];

/// An exact decimal number.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Decimal(rust_decimal::Decimal);

impl Decimal {
    /// The number zero.
    pub const ZERO: Decimal = Decimal(rust_decimal::Decimal::ZERO);

    /// The number one.
    pub const ONE: Decimal = Decimal(rust_decimal::Decimal::ONE);

    /// The largest value: 79,228,162,514,264,337,593,543,950,335.
    pub const MAX_VALUE: Decimal = Decimal(rust_decimal::Decimal::MAX);

    /// The smallest value: -79,228,162,514,264,337,593,543,950,335.
    pub const MIN_VALUE: Decimal = Decimal(rust_decimal::Decimal::MIN);

    /// A decimal of `mantissa / 10^scale`, negated when `negative`.
    ///
    /// Panics when the mantissa has more than 96 bits or the scale is
    /// greater than 28.
    pub fn from_parts(mantissa: u128, scale: u8, negative: bool) -> Decimal {
        assert!(mantissa <= MAX_MANTISSA, "The mantissa of a Decimal has at most 96 bits.");
        assert!(scale <= MAX_SCALE, "Decimal's scale value must be between 0 and 28, inclusive.");
        let mut value = rust_decimal::Decimal::from_i128_with_scale(mantissa as i128, u32::from(scale));
        value.set_sign_negative(negative);
        Decimal(value)
    }

    /// The 96-bit integer that, divided by `10^scale`, is the magnitude.
    #[inline]
    pub fn mantissa(&self) -> u128 {
        self.0.mantissa().unsigned_abs()
    }

    /// The number of digits after the decimal point (0 to 28).
    #[inline]
    pub fn scale(&self) -> u8 {
        self.0.scale() as u8
    }

    /// Whether the sign bit is set. Negative zero has it set and is equal to
    /// zero.
    #[inline]
    pub fn is_sign_negative(&self) -> bool {
        self.0.is_sign_negative()
    }

    /// Whether the value is zero (of any scale and sign).
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// A sum or difference at the scale of the managed type: the larger of
    /// the scales of the operands or, when the digits do not fit at that
    /// scale, the largest scale at which they do.
    fn at_scale_of(result: rust_decimal::Decimal, a: Decimal, b: Decimal) -> Decimal {
        let scale = u32::from(a.scale().max(b.scale()));
        let mut result_scale = result.scale();
        if result_scale >= scale {
            return Decimal(result);
        }

        // Trailing zeros are added while the mantissa stays within 96 bits.
        let mut mantissa = result.mantissa().unsigned_abs();
        while result_scale < scale {
            match mantissa.checked_mul(10) {
                Some(next) if next <= MAX_MANTISSA => {
                    mantissa = next;
                    result_scale += 1;
                }
                _ => break,
            }
        }
        let mut rescaled = rust_decimal::Decimal::from_i128_with_scale(mantissa as i128, result_scale);
        rescaled.set_sign_negative(result.is_sign_negative());
        Decimal(rescaled)
    }

    /// The larger of two values (C# `Math.Max`): of two equal values the
    /// first, which matters for their scales.
    #[inline]
    pub fn max(self, other: Decimal) -> Decimal {
        if self >= other {
            self
        } else {
            other
        }
    }

    /// The smaller of two values (C# `Math.Min`): of two equal values the
    /// second, which matters for their scales.
    #[inline]
    pub fn min(self, other: Decimal) -> Decimal {
        if self < other {
            self
        } else {
            other
        }
    }

    /// The sum, or `None` when it is out of range.
    #[inline]
    pub fn checked_add(self, other: Decimal) -> Option<Decimal> {
        self.0.checked_add(other.0).map(|result| Self::at_scale_of(result, self, other))
    }

    /// The difference, or `None` when it is out of range.
    #[inline]
    pub fn checked_sub(self, other: Decimal) -> Option<Decimal> {
        self.0.checked_sub(other.0).map(|result| Self::at_scale_of(result, self, other))
    }

    /// The quotient, or `None` when it is out of range. Panics when `other`
    /// is zero.
    pub fn checked_div(self, other: Decimal) -> Option<Decimal> {
        assert!(!other.is_zero(), "Attempted to divide by zero.");
        let mut quotient = self.0.checked_div(other.0)?;
        // The scale of the managed type: the difference of the scales when
        // the mantissas divide evenly, else the fewest digits that hold the
        // quotient.
        if self.mantissa() % other.mantissa() == 0 {
            quotient.rescale(u32::from(self.scale().saturating_sub(other.scale())));
            Some(Decimal(quotient))
        } else {
            Some(Decimal(quotient.normalize()))
        }
    }

    /// The nearest double-precision number (the explicit conversion of the
    /// managed type to a double).
    pub fn to_f64(&self) -> f64 {
        // As the managed type converts: the 96-bit mantissa as a double,
        // divided by the power of ten of the scale.
        const TWO_TO_64: f64 = 1.8446744073709552e+019;

        let mantissa = self.mantissa();
        let low64 = mantissa as u64;
        let high = (mantissa >> 64) as u32;
        let dbl = (low64 as f64 + f64::from(high) * TWO_TO_64) / DOUBLE_POWERS_10[usize::from(self.scale())];

        if self.is_sign_negative() {
            -dbl
        } else {
            dbl
        }
    }

    /// The nearest single-precision number (the explicit conversion of the
    /// managed type to a float): the double of the value, narrowed.
    #[inline]
    pub fn to_f32(&self) -> f32 {
        self.to_f64() as f32
    }

    /// The value rounded to an integer, midpoints to even (`Decimal.Round`
    /// with no decimals, which the conversions of the managed runtime to the
    /// integer types apply before their range check).
    pub fn round_to_integer(&self) -> i128 {
        let mantissa = self.mantissa();
        let divisor = 10u128.pow(u32::from(self.scale()));
        let mut integer = mantissa / divisor;
        let remainder = mantissa % divisor;
        // The divisor is even whenever there is a remainder (a scale of zero has none).
        let half = divisor / 2;
        if remainder > half || (remainder == half && remainder != 0 && integer % 2 == 1) {
            integer += 1;
        }

        // At most 2^96, so it fits.
        if self.is_sign_negative() {
            -(integer as i128)
        } else {
            integer as i128
        }
    }

    /// The decimal of a double-precision number, rounded to 15 significant
    /// digits (the explicit conversion of a double to the managed type).
    ///
    /// Panics when the number is not finite or is outside the value range.
    pub fn from_f64(value: f64) -> Decimal {
        Self::try_from_f64(value).unwrap_or_else(|| panic!("{OVERFLOW}"))
    }

    /// The decimal of a single-precision number, rounded to 7 significant
    /// digits (the explicit conversion of a float to the managed type).
    ///
    /// Panics when the number is not finite or is outside the value range.
    pub fn from_f32(value: f32) -> Decimal {
        Self::try_from_f32(value).unwrap_or_else(|| panic!("{OVERFLOW}"))
    }

    /// [`from_f32`](Self::from_f32), with `None` where it panics.
    pub fn try_from_f32(value: f32) -> Option<Decimal> {
        // As for a double: an exponent of -94 could just barely reach 0.5,
        // smaller exponents always round to zero.
        const SNG_BIAS: i32 = 126;
        let exp = ((value.to_bits() >> 23) & 0xFF) as i32 - SNG_BIAS;
        if exp < -94 {
            return Some(Decimal::ZERO);
        }

        // Not a number and the infinities have the largest exponent.
        if exp > 96 {
            return None;
        }

        let negative = value < 0.0;
        let mut dbl = f64::from(value.abs());

        // Round the value to a 7-digit integer: a float has only 7 digits of
        // precision, and the digits beyond are kept out of the decimal.
        let mut power = 6 - ((exp * 19728) >> 16);
        // power is between -22 and 35

        if power >= 0 {
            // Less than 7 digits: scale the value up.
            if power > i32::from(MAX_SCALE) {
                power = i32::from(MAX_SCALE);
            }

            dbl *= DOUBLE_POWERS_10[power as usize];
        } else if power != -1 || dbl >= 1E7 {
            dbl /= DOUBLE_POWERS_10[(-power) as usize];
        } else {
            power = 0; // didn't scale it
        }

        if dbl < 1E6 && power < i32::from(MAX_SCALE) {
            dbl *= 10.0;
            power += 1;
        }

        // Round to an integer, midpoints to even.
        let mut mant = dbl.round_ties_even() as u64;
        if mant == 0 {
            return Some(Decimal::ZERO);
        }

        if power < 0 {
            // Add -power factors of 10, -power <= (29 - 7) = 22.
            Some(Decimal::from_parts(u128::from(mant) * 10u128.pow((-power) as u32), 0, negative))
        } else {
            // Factor out powers of 10 to reduce the scale, if possible. The
            // most that could be factored out is 6: the number has 7 digits
            // and the most significant one is not zero.
            let mut lmax = power.min(6);
            while lmax > 0 && mant % 10 == 0 {
                mant /= 10;
                power -= 1;
                lmax -= 1;
            }

            Some(Decimal::from_parts(u128::from(mant), power as u8, negative))
        }
    }

    /// [`from_f64`](Self::from_f64), with `None` where it panics.
    pub fn try_from_f64(value: f64) -> Option<Decimal> {
        // The most the value can be scaled by is 10^28, which is just
        // slightly more than 2^93. So a double with an exponent of -94 could
        // just barely reach 0.5, but smaller exponents always round to zero.
        const DBL_BIAS: i32 = 1022;
        let exp = ((value.to_bits() >> 52) & 0x7FF) as i32 - DBL_BIAS;
        if exp < -94 {
            return Some(Decimal::ZERO);
        }

        // Not a number and the infinities have the largest exponent.
        if exp > 96 {
            return None;
        }

        let negative = value < 0.0;
        let mut dbl = value.abs();

        // Round the value to a 15-digit integer: a double has only 15 digits
        // of precision, and the digits beyond are kept out of the decimal.
        //
        // The largest power of ten the value could have is the exponent
        // multiplied by log10(2); with scaled integer multiplication,
        // log10(2) * 2^16 = .30103 * 65536 = 19728.3.
        let mut power = 14 - ((exp * 19728) >> 16);
        // power is between -14 and 43

        if power >= 0 {
            // Less than 15 digits: scale the value up.
            if power > i32::from(MAX_SCALE) {
                power = i32::from(MAX_SCALE);
            }

            dbl *= DOUBLE_POWERS_10[power as usize];
        } else if power != -1 || dbl >= 1E15 {
            dbl /= DOUBLE_POWERS_10[(-power) as usize];
        } else {
            power = 0; // didn't scale it
        }

        if dbl < 1E14 && power < i32::from(MAX_SCALE) {
            dbl *= 10.0;
            power += 1;
        }

        // Round to an integer, midpoints to even.
        let mut mant = dbl.round_ties_even() as u64;
        if mant == 0 {
            return Some(Decimal::ZERO);
        }

        if power < 0 {
            // Add -power factors of 10, -power <= (29 - 15) = 14.
            Some(Decimal::from_parts(u128::from(mant) * 10u128.pow((-power) as u32), 0, negative))
        } else {
            // Factor out powers of 10 to reduce the scale, if possible. The
            // most that could be factored out is 14: the number has 15
            // digits and the most significant one is not zero. The scale is
            // never negative, so no more than the power that made the
            // integer can be factored out either.
            let mut lmax = power.min(14);
            while lmax > 0 && mant % 10 == 0 {
                mant /= 10;
                power -= 1;
                lmax -= 1;
            }

            Some(Decimal::from_parts(u128::from(mant), power as u8, negative))
        }
    }

    /// Parses text of the invariant culture with the `Number` styles:
    /// leading and trailing white space, a leading or trailing sign, a
    /// decimal point and group separators.
    pub fn parse(s: &str) -> Result<Decimal, NumberParseError> {
        number_format::parse_decimal(s, NumberStyles::NUMBER, &NumberFormatInfo::invariant_info())
    }

    /// [`Decimal::parse`] without the reason of a failure.
    #[inline]
    pub fn try_parse(s: &str) -> Option<Decimal> {
        Decimal::parse(s).ok()
    }

    /// Parses text with the given styles and format information (C#
    /// `decimal.Parse(string, NumberStyles, IFormatProvider)`). Without a
    /// provider the format information of the current culture is used.
    ///
    /// Styles that are not defined, and the hexadecimal and binary styles,
    /// are an error ([`NumberParseError::InvalidStyle`]).
    pub fn parse_with(
        s: &str,
        styles: NumberStyles,
        provider: Option<&NumberFormatInfo>,
    ) -> Result<Decimal, NumberParseError> {
        match provider {
            Some(info) => number_format::parse_decimal(s, styles, info),
            None => number_format::parse_decimal(s, styles, &NumberFormatInfo::current_info()),
        }
    }

    /// [`Decimal::parse_with`] without the reason of a failure.
    #[inline]
    pub fn try_parse_with(s: &str, styles: NumberStyles, provider: Option<&NumberFormatInfo>) -> Option<Decimal> {
        Decimal::parse_with(s, styles, provider).ok()
    }

    /// Formats the value with a standard or custom numeric format string
    /// (C# `ToString(string, IFormatProvider)`). Without a provider the
    /// format information of the current culture is used.
    pub fn to_string_with(&self, format: &str, provider: Option<&NumberFormatInfo>) -> Result<String, FormatError> {
        match provider {
            Some(info) => number_format::format_decimal(*self, format, info),
            None => number_format::format_decimal(*self, format, &NumberFormatInfo::current_info()),
        }
    }

    /// Formats the value in the general format with the given format
    /// information (C# `ToString(IFormatProvider)`).
    pub fn to_string_provider(&self, provider: &NumberFormatInfo) -> String {
        number_format::format_decimal(*self, "", provider).unwrap_or_default()
    }

    /// Makes the type known to the untyped value conversions: its text
    /// form, its nullable form and the conversions from text and from and
    /// to the other numbers.
    pub(crate) fn register_value_type() {
        ValueTypes::register_display::<Decimal>();
        ValueTypes::register_nullable::<Decimal>();
        ValueTypes::register_parse::<Decimal>(|s| Decimal::parse(s).ok());
        ValueTypes::register_conversion::<String, Option<Decimal>>(|s| Decimal::parse(s).ok().map(Some));
        ValueTypes::register_conversion::<&'static str, Decimal>(|s| Decimal::parse(s).ok());
        ValueTypes::register_conversion::<&'static str, Option<Decimal>>(|s| Decimal::parse(s).ok().map(Some));

        macro_rules! integers {
            ($($ty:ty),*) => {
                $(
                    ValueTypes::register_conversion::<$ty, Decimal>(|v| Some(Decimal::from(*v)));
                    ValueTypes::register_conversion::<$ty, Option<Decimal>>(|v| Some(Some(Decimal::from(*v))));
                )*
            };
        }
        integers!(i8, i16, i32, i64, u8, u16, u32, u64, isize, usize);

        // The floating point numbers convert when they are in the range of
        // the type (`Convert.ToDecimal` fails for the others, and for the
        // infinities and not-a-number).
        ValueTypes::register_conversion::<f64, Decimal>(|v| Decimal::try_from_f64(*v));
        ValueTypes::register_conversion::<f64, Option<Decimal>>(|v| Decimal::try_from_f64(*v).map(Some));
        ValueTypes::register_conversion::<f32, Decimal>(|v| Decimal::try_from_f32(*v));
        ValueTypes::register_conversion::<f32, Option<Decimal>>(|v| Decimal::try_from_f32(*v).map(Some));
        ValueTypes::register_conversion::<Decimal, f64>(|v| Some(v.to_f64()));
        ValueTypes::register_conversion::<Decimal, Option<f64>>(|v| Some(Some(v.to_f64())));
        ValueTypes::register_conversion::<Decimal, f32>(|v| Some(v.to_f32()));
        ValueTypes::register_conversion::<Decimal, Option<f32>>(|v| Some(Some(v.to_f32())));

        // A decimal becomes an integer rounded, midpoints to even, when the
        // integer is in the range of the type (`Convert.ToInt32(decimal)`
        // and its siblings).
        macro_rules! to_integers {
            ($($ty:ty),*) => {
                $(
                    ValueTypes::register_conversion::<Decimal, $ty>(|v| <$ty>::try_from(v.round_to_integer()).ok());
                    ValueTypes::register_conversion::<Decimal, Option<$ty>>(|v| {
                        <$ty>::try_from(v.round_to_integer()).ok().map(Some)
                    });
                )*
            };
        }
        to_integers!(i8, i16, i32, i64, u8, u16, u32, u64, isize, usize);
    }
}

crate::ferro_markup_type!(struct Decimal {
    namespace: "System",
    handles: [Decimal],
    parse: Decimal::parse,
    constructors: [() => Decimal::default],
});

impl Default for Decimal {
    #[inline]
    fn default() -> Self {
        Decimal::ZERO
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The general format of the invariant culture: every digit of the
        // value, trailing zeros included; negative zero without its sign.
        f.pad(&self.to_string_provider(&NumberFormatInfo::invariant_info()))
    }
}

impl fmt::Debug for Decimal {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl FromStr for Decimal {
    type Err = NumberParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Decimal::parse(s)
    }
}

macro_rules! from_integers {
    ($($ty:ty),*) => {
        $(
            impl From<$ty> for Decimal {
                #[inline]
                fn from(value: $ty) -> Decimal {
                    Decimal(rust_decimal::Decimal::from(value))
                }
            }
        )*
    };
}

from_integers!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl Neg for Decimal {
    type Output = Decimal;

    #[inline]
    fn neg(self) -> Decimal {
        Decimal(-self.0)
    }
}

macro_rules! operators {
    ($($trait_:ident $method:ident $checked:ident;)*) => {
        $(
            impl $trait_ for Decimal {
                type Output = Decimal;

                #[inline]
                fn $method(self, other: Decimal) -> Decimal {
                    self.$checked(other).expect(OVERFLOW)
                }
            }
        )*
    };
}

operators! {
    Add add checked_add;
    Sub sub checked_sub;
    Div div checked_div;
}

impl AddAssign for Decimal {
    #[inline]
    fn add_assign(&mut self, other: Decimal) {
        *self = *self + other;
    }
}

impl SubAssign for Decimal {
    #[inline]
    fn sub_assign(&mut self, other: Decimal) {
        *self = *self - other;
    }
}

#[cfg(test)]
mod tests {
    mod double_conversion_data {
        include!("decimal_double_conversion_data.rs");
    }

    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn d(text: &str) -> Decimal {
        Decimal::parse(text).unwrap()
    }

    fn hash_of(value: Decimal) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn double_conversions_fail_outside_the_value_range() {
        assert!(std::panic::catch_unwind(|| Decimal::from_f64(f64::NAN)).is_err());
        assert!(std::panic::catch_unwind(|| Decimal::from_f64(1e30)).is_err());
        assert!(std::panic::catch_unwind(|| Decimal::from_f64(f64::INFINITY)).is_err());
        assert!(!Decimal::from_f64(-0.0).is_sign_negative());
        assert!(!Decimal::from_f64(-1e-30).is_sign_negative());
    }

    #[test]
    fn conversion_from_double_matches_the_managed_runtime() {
        for &(bits, expected) in double_conversion_data::FROM_DOUBLE {
            let value = f64::from_bits(bits);
            if expected == "!" {
                assert!(std::panic::catch_unwind(|| Decimal::from_f64(value)).is_err(), "{value:e}");
            } else {
                assert_eq!(Decimal::from_f64(value).to_string(), expected, "{value:e} ({bits:#018X})");
            }
        }
    }

    #[test]
    fn conversion_to_double_matches_the_managed_runtime() {
        for &(text, bits) in double_conversion_data::TO_DOUBLE {
            let actual = d(text).to_f64();
            assert_eq!(actual.to_bits(), bits, "{text}: {actual:e} instead of {:e}", f64::from_bits(bits));
        }
    }

    #[test]
    fn constants_and_parts() {
        assert_eq!(Decimal::MAX_VALUE.to_string(), "79228162514264337593543950335");
        assert_eq!(Decimal::MIN_VALUE.to_string(), "-79228162514264337593543950335");
        assert_eq!(Decimal::ZERO.to_string(), "0");
        assert_eq!(Decimal::ONE.to_string(), "1");
        assert_eq!(Decimal::from_parts(12345, 3, true).to_string(), "-12.345");
        assert_eq!(Decimal::from_parts(5, 3, false).to_string(), "0.005");
        assert_eq!(Decimal::from_parts(110, 2, false).mantissa(), 110);
        assert_eq!(Decimal::from_parts(110, 2, false).scale(), 2);
        assert_eq!(Decimal::MAX_VALUE.mantissa(), MAX_MANTISSA);
        assert!(Decimal::from_parts(0, 1, true).is_sign_negative());
        assert!(Decimal::from_parts(0, 1, true).is_zero());
        assert_eq!(Decimal::default(), Decimal::ZERO);
    }

    #[test]
    fn display_preserves_trailing_zeros() {
        assert_eq!(d("1.10").to_string(), "1.10");
        assert_eq!(d("1.1").to_string(), "1.1");
        assert_eq!(d("0.00").to_string(), "0.00");
        assert_eq!(d("-0.0").to_string(), "0.0");
        assert_eq!(d("100").to_string(), "100");
        assert_eq!(d("0.0000000000000000000000000001").to_string(), "0.0000000000000000000000000001");
        assert_eq!(format!("{:>6}", d("1.5")), "   1.5");
        assert_eq!(format!("{:?}", d("1.50")), "1.50");
    }

    #[test]
    fn equality_ordering_and_hash_are_by_value() {
        assert_eq!(d("1.10"), d("1.1"));
        assert_eq!(hash_of(d("1.10")), hash_of(d("1.1")));
        assert_eq!(d("0.00"), Decimal::ZERO);
        assert_eq!(d("-0"), Decimal::ZERO);
        assert_eq!(hash_of(d("-0.00")), hash_of(Decimal::ZERO));
        assert!(d("1.1") < d("1.11"));
        assert!(d("-1.1") > d("-1.11"));
        assert!(d("-1") < d("0.5"));
        assert!(Decimal::MIN_VALUE < Decimal::MAX_VALUE);
        assert!(Decimal::MAX_VALUE > d("0.0000000000000000000000000001"));
        assert_eq!(d("2").max(d("3.0")).to_string(), "3.0");
        assert_eq!(d("5").min(d("3.0")).to_string(), "3.0");
        assert_eq!(d("5").clamp(d("200"), d("400")), d("200"));
    }

    #[test]
    fn addition_and_subtraction_take_the_larger_scale() {
        assert_eq!((d("1.10") + d("2.2")).to_string(), "3.30");
        assert_eq!((d("1.1") - d("1.10")).to_string(), "0.00");
        assert_eq!((d("1") - d("3")).to_string(), "-2");
        assert_eq!((d("-1.5") + d("1.5")).to_string(), "0.0");
        assert_eq!((d("0.1") + d("0.2")), d("0.3"));
        assert_eq!((d("5") + Decimal::ONE).to_string(), "6");
        assert_eq!((d("10.11") + Decimal::ONE).to_string(), "11.11");
        assert_eq!((Decimal::MAX_VALUE - Decimal::ONE).to_string(), "79228162514264337593543950334");
        let mut value = d("1.5");
        value += d("1");
        value -= d("0.25");
        assert_eq!(value.to_string(), "2.25");
        assert_eq!((-d("1.50")).to_string(), "-1.50");
    }

    #[test]
    fn a_sum_with_a_zero_of_a_larger_scale_takes_the_largest_scale_that_fits() {
        // Results of the managed runtime.
        assert_eq!(
            (d("0.000000000000000000000000000") + d("857337436073678641268323889")).to_string(),
            "857337436073678641268323889.0"
        );
        assert_eq!(
            (d("0.000000000000000000000000000") - d("857337436073678641268323889")).to_string(),
            "-857337436073678641268323889.0"
        );
        assert_eq!(
            (d("434.559") + d("0.0000000000000000000000000000")).to_string(),
            "434.55900000000000000000000000"
        );
        assert_eq!(
            (d("-4128645094226724.54") - d("-0.00000000000000000000")).to_string(),
            "-4128645094226724.5400000000000"
        );
        assert_eq!((Decimal::MAX_VALUE + d("0.00")).to_string(), "79228162514264337593543950335");
        assert_eq!((d("5") + d("0.00")).to_string(), "5.00");
    }

    #[test]
    fn max_and_min_of_equal_values_are_those_of_the_managed_runtime() {
        // Math.Max returns the first of two equal values, Math.Min the second.
        assert_eq!(d("5.00").max(d("5")).to_string(), "5.00");
        assert_eq!(d("5").max(d("5.00")).to_string(), "5");
        assert_eq!(d("5.00").min(d("5")).to_string(), "5");
        assert_eq!(d("5").min(d("5.00")).to_string(), "5.00");
        assert_eq!(d("-0.0").max(d("0")).to_string(), "0.0");
        assert_eq!(d("2").max(d("3.0")).to_string(), "3.0");
        assert_eq!(d("5").min(d("3.0")).to_string(), "3.0");
    }

    #[test]
    #[should_panic(expected = "Value was either too large or too small for a Decimal.")]
    fn addition_overflow_panics() {
        let _ = Decimal::MAX_VALUE + Decimal::ONE;
    }

    #[test]
    fn checked_operations_report_overflow() {
        assert_eq!(Decimal::MAX_VALUE.checked_add(Decimal::ONE), None);
        assert_eq!(Decimal::MIN_VALUE.checked_sub(Decimal::ONE), None);
        assert_eq!(Decimal::MAX_VALUE.checked_div(d("0.1")), None);
        assert_eq!(Decimal::MAX_VALUE.checked_add(Decimal::MIN_VALUE), Some(Decimal::ZERO));
    }

    #[test]
    fn division() {
        // The percent text of the control is divided by one hundred.
        assert_eq!((d("10.11") / Decimal::from(100)).to_string(), "0.1011");
        assert_eq!((d("1011.00") / Decimal::from(100)).to_string(), "10.11");
        assert_eq!((d("50") / Decimal::from(100)).to_string(), "0.5");
        assert_eq!((d("-1") / d("4")).to_string(), "-0.25");
        assert_eq!((d("1") / d("3")).to_string(), "0.3333333333333333333333333333");
        assert_eq!((d("6.00") / d("2")).to_string(), "3.00");
        assert_eq!((d("6") / d("2.00")).to_string(), "3");
        assert_eq!((d("1.00") / d("8")).to_string(), "0.125");
        assert_eq!((d("0") / d("7")).to_string(), "0");
    }

    #[test]
    #[should_panic(expected = "Attempted to divide by zero.")]
    fn division_by_zero_panics() {
        let _ = Decimal::ONE / Decimal::ZERO;
    }

    #[test]
    fn integer_conversions() {
        assert_eq!(Decimal::from(42i32).to_string(), "42");
        assert_eq!(Decimal::from(-42i64).to_string(), "-42");
        assert_eq!(Decimal::from(i64::MIN).to_string(), "-9223372036854775808");
        assert_eq!(Decimal::from(u64::MAX).to_string(), "18446744073709551615");
    }

    #[test]
    fn single_conversions() {
        // Seven significant digits.
        assert_eq!(Decimal::from_f32(0.1).to_string(), "0.1");
        assert_eq!(Decimal::from_f32(-2.5).to_string(), "-2.5");
        assert_eq!(Decimal::from_f32(1234.5).to_string(), "1234.5");
        assert_eq!(Decimal::from_f32(16777216.0).to_string(), "16777220");
        assert_eq!(Decimal::from_f32(1e10).to_string(), "10000000000");
        assert_eq!(Decimal::from_f32(0.0).to_string(), "0");
        assert!(!Decimal::from_f32(-1e-30).is_sign_negative());
        assert_eq!(Decimal::try_from_f32(f32::NAN), None);
        assert_eq!(Decimal::try_from_f32(f32::INFINITY), None);
        assert_eq!(Decimal::try_from_f32(1e30), None);
        assert!(std::panic::catch_unwind(|| Decimal::from_f32(f32::NEG_INFINITY)).is_err());
        assert_eq!(Decimal::try_from_f64(f64::NAN), None);
        assert_eq!(Decimal::try_from_f64(-1e30), None);
        assert_eq!(Decimal::try_from_f64(2.5), Some(d("2.5")));

        assert_eq!(d("0.5").to_f32(), 0.5);
        assert_eq!(d("0.1").to_f32(), 0.1f32);
        assert_eq!(Decimal::MAX_VALUE.to_f32(), 7.9228163e28);
    }

    #[test]
    fn rounding_to_an_integer_takes_midpoints_to_even() {
        assert_eq!(d("0").round_to_integer(), 0);
        assert_eq!(d("7").round_to_integer(), 7);
        assert_eq!(d("0.5").round_to_integer(), 0);
        assert_eq!(d("1.5").round_to_integer(), 2);
        assert_eq!(d("2.5").round_to_integer(), 2);
        assert_eq!(d("2.50").round_to_integer(), 2);
        assert_eq!(d("2.51").round_to_integer(), 3);
        assert_eq!(d("2.49").round_to_integer(), 2);
        assert_eq!(d("-1.5").round_to_integer(), -2);
        assert_eq!(d("-2.5").round_to_integer(), -2);
        assert_eq!(d("-0.4").round_to_integer(), 0);
        assert_eq!(Decimal::MAX_VALUE.round_to_integer(), MAX_MANTISSA as i128);
        assert_eq!(Decimal::MIN_VALUE.round_to_integer(), -(MAX_MANTISSA as i128));
    }

    #[test]
    fn parse_and_from_str() {
        assert_eq!("1.10".parse::<Decimal>().unwrap().to_string(), "1.10");
        assert_eq!(d(" -1,234.50 ").to_string(), "-1234.50");
        assert_eq!(d("5-").to_string(), "-5");
        assert_eq!(d("+5").to_string(), "5");
        assert_eq!(d(".5").to_string(), "0.5");
        assert_eq!(d("5.").to_string(), "5");
        assert_eq!(Decimal::parse(""), Err(NumberParseError::Format));
        assert_eq!(Decimal::parse("abc"), Err(NumberParseError::Format));
        assert_eq!(Decimal::parse("1e2"), Err(NumberParseError::Format));
        assert_eq!(Decimal::parse("$5"), Err(NumberParseError::Format));
        assert_eq!(Decimal::parse("79228162514264337593543950336"), Err(NumberParseError::Overflow));
        assert_eq!(Decimal::try_parse("x"), None);
        assert_eq!(d("79228162514264337593543950335"), Decimal::MAX_VALUE);
        assert_eq!(d("-79228162514264337593543950335"), Decimal::MIN_VALUE);
        // More digits than the precision holds are rounded half to even.
        assert_eq!(d("0.00000000000000000000000000005").to_string(), "0.0000000000000000000000000000");
        assert_eq!(d("0.00000000000000000000000000015").to_string(), "0.0000000000000000000000000002");
        assert_eq!(d("0.000000000000000000000000000051").to_string(), "0.0000000000000000000000000001");
        assert_eq!(d("1.00000000000000000000000000005").to_string(), "1.0000000000000000000000000000");
        assert_eq!(d("7922816251426433759354395033.55").to_string(), "7922816251426433759354395034");
    }

    #[test]
    fn untyped_conversions() {
        use crate::data::core::ValueType;
        use crate::BoxedValue;
        use std::rc::Rc;

        let convert = |value: BoxedValue, target: ValueType| ValueTypes::try_convert_registered(&value, target);
        let text: BoxedValue = Rc::new("1.50".to_string());
        let converted = convert(text, ValueType::of::<Decimal>()).unwrap();
        assert_eq!(converted.downcast_ref::<Decimal>().unwrap().to_string(), "1.50");

        let number: BoxedValue = Rc::new(7i32);
        let converted = convert(number, ValueType::of::<Option<Decimal>>()).unwrap();
        assert_eq!(converted.downcast_ref::<Option<Decimal>>(), Some(&Some(d("7"))));

        let value: BoxedValue = Rc::new(d("2.5"));
        assert_eq!(
            convert(value.clone(), ValueType::of::<Option<Decimal>>()).unwrap().downcast_ref::<Option<Decimal>>(),
            Some(&Some(d("2.5")))
        );
        assert_eq!(ValueTypes::to_display_string(Some(&value)), "2.5");

        assert!(ValueTypes::accepts_null(ValueType::of::<Option<Decimal>>()));
        assert!(!ValueTypes::accepts_null(ValueType::of::<Decimal>()));
    }
}
