//! Scalar helpers shared by the numerics types: invariant formatting of a
//! single-precision float, NaN-propagating minimum and maximum, and the IEEE
//! remainder.

use std::fmt::{self, Write};

use crate::utilities::math_utilities::{max_f32, min_f32};

/// Small stack buffer used to post-process `{:e}` output without allocating.
struct StackBuf {
    bytes: [u8; 32],
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

/// `Display` adapter printing a single-precision float the way the reference
/// runtime does under the invariant culture: the shortest round-trippable
/// digits, plain notation for decimal exponents in `-4..=8`, scientific
/// notation (`1E+09`, `1E-05`) otherwise, and `NaN` / `Infinity` /
/// `-Infinity` for the non-finite values.
#[derive(Clone, Copy, Debug)]
pub struct InvariantF32(pub f32);

impl fmt::Display for InvariantF32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.0;
        if value.is_nan() {
            return f.write_str("NaN");
        }
        if value.is_infinite() {
            return f.write_str(if value > 0.0 { "Infinity" } else { "-Infinity" });
        }
        if value == 0.0 {
            return write!(f, "{}", value);
        }

        let mut buf = StackBuf {
            bytes: [0; 32],
            len: 0,
        };
        write!(buf, "{:e}", value)?;
        let text = std::str::from_utf8(&buf.bytes[..buf.len]).map_err(|_| fmt::Error)?;
        let (mantissa, exponent) = text.split_once('e').ok_or(fmt::Error)?;
        let exponent: i32 = exponent.parse().map_err(|_| fmt::Error)?;
        if (-4..=8).contains(&exponent) {
            return write!(f, "{}", value);
        }
        write!(
            f,
            "{}E{}{:02}",
            mantissa,
            if exponent < 0 { '-' } else { '+' },
            exponent.unsigned_abs()
        )
    }
}

/// The larger of two floats; NaN is propagated and `+0.0 > -0.0`.
#[inline]
pub(crate) fn max(value1: f32, value2: f32) -> f32 {
    max_f32(value1, value2)
}

/// The smaller of two floats; NaN is propagated and `-0.0 < +0.0`.
#[inline]
pub(crate) fn min(value1: f32, value2: f32) -> f32 {
    min_f32(value1, value2)
}

/// The remainder of `x / y` as specified by IEEE 754 (the quotient is rounded
/// to the nearest integer, ties to even).
pub(crate) fn ieee_remainder(x: f32, y: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if y.is_nan() {
        return y;
    }

    let regular_mod = x % y;
    if regular_mod.is_nan() {
        return f32::NAN;
    }
    if regular_mod == 0.0 && x.is_sign_negative() {
        return -0.0;
    }

    // `x` is neither NaN nor zero here (a zero `x` gives a zero remainder).
    let sign = if x > 0.0 { 1.0 } else { -1.0 };
    let alternative_result = regular_mod - (y.abs() * sign);
    if alternative_result.abs() == regular_mod.abs() {
        let division_result = x / y;
        let rounded_result = division_result.round_ties_even();
        if rounded_result.abs() > division_result.abs() {
            return alternative_result;
        }
        return regular_mod;
    }
    if alternative_result.abs() < regular_mod.abs() {
        return alternative_result;
    }
    regular_mod
}
