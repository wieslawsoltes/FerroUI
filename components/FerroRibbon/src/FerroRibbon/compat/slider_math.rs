//! Exact pinned Uno Slider/DirectUI numerical rules, independent of rendering.
//! Copyright Microsoft Corporation, MIT
const DOUBLE_EPSILON: f64 = f64::from_bits(1);
// DirectUI Max/Min deliberately use conditional comparisons, not Math.Max/Min.
pub(crate) fn max(a: f64, b: f64) -> f64 {
    if a >= b { a } else { b }
}
pub(crate) fn min(a: f64, b: f64) -> f64 {
    if a <= b { a } else { b }
}
pub(crate) fn are_close(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    let epsilon = (a.abs() + b.abs() + 10.) * DOUBLE_EPSILON;
    let delta = a - b;
    -epsilon < delta && epsilon > delta
}
pub(crate) fn fractional(v: f64) -> f64 {
    if v > 0. { v - v.floor() } else { v - v.ceil() }
}
pub(crate) fn closest_step(minimum: f64, maximum: f64, step: f64, value: f64) -> f64 {
    let steps = value / step;
    let next = min(maximum, steps.ceil() * step);
    let previous = max(minimum, steps.floor() * step);
    let (a, b) = (next - value, value - previous);
    let result = if a < b && !are_close(a, b) {
        next
    } else {
        previous
    };
    if !result.is_nan() {
        return result;
    }
    // Match the source fixture's quiet NaN for invalid arithmetic. Preserve
    // the sign and payload of an explicit NaN passed into the calculation.
    let input_nan = if value.is_nan() { value } else { step };
    if input_nan.is_nan() {
        f64::from_bits(input_nan.to_bits() | (1 << 51))
    } else {
        f64::NAN
    }
}
pub(crate) fn step_value(minimum: f64, maximum: f64, value: f64, step: f64, forward: bool) -> f64 {
    if !forward && are_close(value, maximum) && !are_close(fractional(value / step), 0.) {
        (value / step).floor() * step
    } else {
        closest_step(
            minimum,
            maximum,
            step,
            if forward { value + step } else { value - step },
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_uno_closest_step_matches_exact_bits_including_nonfinite_inputs() {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/slider.json")).unwrap();
        let number = |s: &serde_json::Value| {
            f64::from_bits(u64::from_str_radix(s.as_str().unwrap(), 16).unwrap())
        };
        for row in v["corpus"]["math"].as_array().unwrap() {
            let result = closest_step(
                number(&row["minimum"]),
                number(&row["maximum"]),
                number(&row["step"]),
                number(&row["input"]),
            );
            assert_eq!(
                format!("{:016X}", result.to_bits()),
                row["result"].as_str().unwrap(),
                "{row}"
            );
        }
    }
}
