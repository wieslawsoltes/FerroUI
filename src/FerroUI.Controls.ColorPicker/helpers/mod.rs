//! The helpers of the colour picker (`Helpers/` of the upstream project):
//! the public [`ColorHelper`] and the internal structures and algorithms
//! the controls share.

mod color_helper;
pub(crate) mod color_picker_helpers;
pub(crate) mod hsv;
pub(crate) mod increment_amount;
pub(crate) mod increment_direction;
pub(crate) mod rgb;

pub use color_helper::ColorHelper;

/// `Math.Round(value, digits, mode)` of the .NET base library: the value is
/// scaled by the power of ten, rounded to an integer (ties to even, or away
/// from zero when `away_from_zero`) and scaled back.
pub(crate) fn round_digits(value: f64, digits: i32, away_from_zero: bool) -> f64 {
    // The .NET implementation leaves values whose magnitude is at least 1e16 unchanged.
    if value.abs() >= 1e16 {
        return value;
    }

    let power10 = 10f64.powi(digits);
    let mut value = value * power10;

    if away_from_zero {
        let fraction = value.fract();
        value = value.trunc();
        if fraction.abs() >= 0.5 {
            value += fraction.signum();
        }
    } else {
        value = value.round_ties_even();
    }

    value / power10
}

#[cfg(test)]
mod tests {
    // Not from upstream: checks the counterpart of the .NET rounding the helpers use.
    use super::round_digits;

    #[test]
    fn round_digits_rounds_ties_to_even_or_away_from_zero() {
        assert_eq!(2.0, round_digits(2.5, 0, false));
        assert_eq!(3.0, round_digits(2.5, 0, true));
        assert_eq!(-3.0, round_digits(-2.5, 0, true));
        assert_eq!(0.2, round_digits(0.25, 1, false));
        assert_eq!(0.13, round_digits(0.125, 2, true));
    }
}
