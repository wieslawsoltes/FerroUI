// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use std::fmt;

/// A point in docking coordinates.
#[derive(Clone, Copy, Debug, Default)]
pub struct DockPoint {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
}

impl DockPoint {
    /// Initializes a point with the supplied coordinates.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl PartialEq for DockPoint {
    fn eq(&self, other: &Self) -> bool {
        double_equals(self.x, other.x) && double_equals(self.y, other.y)
    }
}

impl fmt::Display for DockPoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}, {}",
            InvariantDouble(self.x),
            InvariantDouble(self.y)
        )
    }
}

// Double.Equals, used by value-type equality, treats NaN as equal to NaN.
pub(crate) fn double_equals(left: f64, right: f64) -> bool {
    left == right || (left.is_nan() && right.is_nan())
}

// The invariant general format used by the source's String.Format. Rust's
// Display uses different exponent thresholds and infinity/exponent spelling.
pub(crate) struct InvariantDouble(pub f64);

impl fmt::Display for InvariantDouble {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.0;
        if value.is_nan() {
            return formatter.write_str("NaN");
        }
        if value.is_infinite() {
            return formatter.write_str(if value.is_sign_negative() {
                "-Infinity"
            } else {
                "Infinity"
            });
        }
        if value == 0.0 {
            return formatter.write_str(if value.is_sign_negative() { "-0" } else { "0" });
        }
        // Ryū uses round-to-even for the last shortest digit, matching the source.
        // The standard Rust formatter differs at exact decimal half-way cases.
        let mut buffer = ryu::Buffer::new();
        let formatted = buffer.format_finite(value);
        let formatted = if let Some(unsigned) = formatted.strip_prefix('-') {
            formatter.write_str("-")?;
            unsigned
        } else {
            formatted
        };
        let (mantissa, explicit_exponent) = match formatted.split_once('e') {
            Some((mantissa, exponent)) => {
                (mantissa, exponent.parse::<i32>().map_err(|_| fmt::Error)?)
            }
            None => (formatted, 0),
        };
        let decimal_position = mantissa.find('.').unwrap_or(mantissa.len());
        let first_nonzero = mantissa
            .bytes()
            .position(|byte| byte != b'0' && byte != b'.')
            .ok_or(fmt::Error)?;
        let leading_decimal_point = usize::from(first_nonzero > decimal_position);
        let exponent = explicit_exponent + decimal_position as i32
            - (first_nonzero - leading_decimal_point) as i32
            - 1;
        let mantissa = mantissa[first_nonzero..]
            .trim_end_matches('0')
            .trim_end_matches('.');
        let mut digits = [0u8; 17];
        let mut count = 0;
        for byte in mantissa.bytes() {
            if byte != b'.' {
                *digits.get_mut(count).ok_or(fmt::Error)? = byte;
                count += 1;
            }
        }
        if !(-4..17).contains(&exponent) {
            formatter.write_str(std::str::from_utf8(&digits[..1]).map_err(|_| fmt::Error)?)?;
            if count > 1 {
                formatter.write_str(".")?;
                formatter
                    .write_str(std::str::from_utf8(&digits[1..count]).map_err(|_| fmt::Error)?)?;
            }
            return write!(formatter, "E{exponent:+03}");
        }
        if exponent < 0 {
            formatter.write_str("0.")?;
            for _ in 0..(-exponent - 1) {
                formatter.write_str("0")?;
            }
            return formatter
                .write_str(std::str::from_utf8(&digits[..count]).map_err(|_| fmt::Error)?);
        }
        let decimal = exponent as usize + 1;
        for index in 0..count.max(decimal) {
            if index == decimal {
                formatter.write_str(".")?;
            }
            let digit = if index < count {
                &digits[index..index + 1]
            } else {
                b"0"
            };
            formatter.write_str(std::str::from_utf8(digit).map_err(|_| fmt::Error)?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_preserves_coordinates_default_and_value_copy_semantics() {
        let point = DockPoint::new(-12.5, 0.25);
        assert_eq!(point.x, -12.5);
        assert_eq!(point.y, 0.25);
        assert_eq!(point.to_string(), "-12.5, 0.25");
        assert_eq!(DockPoint::default(), DockPoint::new(0.0, 0.0));
        let mut copy = point;
        copy.x = 42.0;
        assert_eq!(point.x, -12.5);
        assert_ne!(point, copy);
    }

    #[test]
    fn value_equality_uses_double_equals_for_nan_and_signed_zero() {
        let nan = f64::from_bits(0xfff8_0000_0000_0001);
        assert_eq!(DockPoint::new(f64::NAN, -0.0), DockPoint::new(nan, 0.0));
        assert_ne!(DockPoint::new(0.0, 1.0), DockPoint::new(0.0, 2.0));
    }

    #[test]
    fn invariant_format_preserves_special_values_and_exponent_boundaries() {
        assert_eq!(
            DockPoint::new(f64::NAN, f64::NEG_INFINITY).to_string(),
            "NaN, -Infinity"
        );
        assert_eq!(
            DockPoint::new(f64::INFINITY, -0.0).to_string(),
            "Infinity, -0"
        );
        assert_eq!(DockPoint::new(1e-4, 1e-5).to_string(), "0.0001, 1E-05");
        assert_eq!(
            DockPoint::new(1e16, 1e17).to_string(),
            "10000000000000000, 1E+17"
        );
        assert_eq!(
            DockPoint::new(f64::from_bits(1), f64::MAX).to_string(),
            "5E-324, 1.7976931348623157E+308"
        );
    }

    #[test]
    fn numeric_format_matches_the_frozen_dotnet_reference() {
        for line in include_str!("../../../../tests/Dock/data/invariant-double.tsv").lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let (bits, expected) = line.split_once('\t').unwrap();
            let bits = u64::from_str_radix(bits, 16).unwrap();
            let number = f64::from_bits(bits);
            assert_eq!(
                InvariantDouble(number).to_string(),
                expected,
                "bits: {bits:016X}"
            );
        }
    }
}
