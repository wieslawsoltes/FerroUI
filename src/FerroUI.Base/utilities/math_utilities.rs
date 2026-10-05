//! Math utilities not provided by the standard library.

use crate::{Point, Vector};

/// Returns the smaller of two doubles; NaN is propagated and `-0.0 < +0.0`.
///
/// `f64::min` ignores NaN operands, which is not what the layout and geometry code
/// ported into this crate expects.
#[inline]
pub fn min(val1: f64, val2: f64) -> f64 {
    if val1 != val2 {
        if !val1.is_nan() {
            return if val1 < val2 { val1 } else { val2 };
        }
        return val1;
    }
    if val1.is_sign_negative() {
        val1
    } else {
        val2
    }
}

/// Returns the larger of two doubles; NaN is propagated and `+0.0 > -0.0`.
#[inline]
pub fn max(val1: f64, val2: f64) -> f64 {
    if val1 != val2 {
        if !val1.is_nan() {
            return if val2 < val1 { val1 } else { val2 };
        }
        return val1;
    }
    if val2.is_sign_negative() {
        val1
    } else {
        val2
    }
}

/// Returns the smaller of two floats; NaN is propagated and `-0.0 < +0.0`.
#[inline]
pub fn min_f32(val1: f32, val2: f32) -> f32 {
    if val1 != val2 {
        if !val1.is_nan() {
            return if val1 < val2 { val1 } else { val2 };
        }
        return val1;
    }
    if val1.is_sign_negative() {
        val1
    } else {
        val2
    }
}

/// Returns the larger of two floats; NaN is propagated and `+0.0 > -0.0`.
#[inline]
pub fn max_f32(val1: f32, val2: f32) -> f32 {
    if val1 != val2 {
        if !val1.is_nan() {
            return if val2 < val1 { val1 } else { val2 };
        }
        return val1;
    }
    if val2.is_sign_negative() {
        val1
    } else {
        val2
    }
}

/// Equality in which NaN equals NaN (and `0.0 == -0.0`), as opposed to `==`.
#[inline]
pub fn equals(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

/// Provides math utilities not provided by the standard library.
pub struct MathUtilities;

impl MathUtilities {
    /// Smallest value such that `1.0 + DOUBLE_EPSILON != 1.0`.
    pub const DOUBLE_EPSILON: f64 = f64::EPSILON; // 2.2204460492503131e-16

    pub const FLOAT_EPSILON: f32 = f32::EPSILON; // 1.192092896e-07

    /// Returns whether or not two doubles are "close", i.e. within a relative epsilon
    /// of each other.
    #[inline]
    pub fn are_close(value1: f64, value2: f64) -> bool {
        // in case they are Infinities (then epsilon check does not work)
        if value1 == value2 {
            return true;
        }
        let eps = (value1.abs() + value2.abs() + 10.0) * Self::DOUBLE_EPSILON;
        let delta = value1 - value2;
        (-eps < delta) && (eps > delta)
    }

    /// Returns whether or not two doubles are within the fixed epsilon `eps` of each other.
    #[inline]
    pub fn are_close_eps(value1: f64, value2: f64, eps: f64) -> bool {
        // in case they are Infinities (then epsilon check does not work)
        if value1 == value2 {
            return true;
        }
        let delta = value1 - value2;
        (-eps < delta) && (eps > delta)
    }

    /// Returns whether or not two floats are "close".
    #[inline]
    pub fn are_close_f32(value1: f32, value2: f32) -> bool {
        // in case they are Infinities (then epsilon check does not work)
        if value1 == value2 {
            return true;
        }
        let eps = (value1.abs() + value2.abs() + 10.0) * Self::FLOAT_EPSILON;
        let delta = value1 - value2;
        (-eps < delta) && (eps > delta)
    }

    /// Returns whether the first double is strictly less than *and* not close to the second.
    #[inline]
    pub fn less_than(value1: f64, value2: f64) -> bool {
        (value1 < value2) && !Self::are_close(value1, value2)
    }

    /// Returns whether the first float is strictly less than *and* not close to the second.
    #[inline]
    pub fn less_than_f32(value1: f32, value2: f32) -> bool {
        (value1 < value2) && !Self::are_close_f32(value1, value2)
    }

    /// Returns whether the first double is strictly greater than *and* not close to the second.
    #[inline]
    pub fn greater_than(value1: f64, value2: f64) -> bool {
        (value1 > value2) && !Self::are_close(value1, value2)
    }

    /// Returns whether the first float is strictly greater than *and* not close to the second.
    #[inline]
    pub fn greater_than_f32(value1: f32, value2: f32) -> bool {
        (value1 > value2) && !Self::are_close_f32(value1, value2)
    }

    /// Returns whether the first double is less than or close to the second.
    #[inline]
    pub fn less_than_or_close(value1: f64, value2: f64) -> bool {
        (value1 < value2) || Self::are_close(value1, value2)
    }

    /// Returns whether the first float is less than or close to the second.
    #[inline]
    pub fn less_than_or_close_f32(value1: f32, value2: f32) -> bool {
        (value1 < value2) || Self::are_close_f32(value1, value2)
    }

    /// Returns whether the first double is greater than or close to the second.
    #[inline]
    pub fn greater_than_or_close(value1: f64, value2: f64) -> bool {
        (value1 > value2) || Self::are_close(value1, value2)
    }

    /// Returns whether the first float is greater than or close to the second.
    #[inline]
    pub fn greater_than_or_close_f32(value1: f32, value2: f32) -> bool {
        (value1 > value2) || Self::are_close_f32(value1, value2)
    }

    /// Returns whether the double is "close" to 1. Same as `are_close(value, 1.0)` but faster.
    #[inline]
    pub fn is_one(value: f64) -> bool {
        (value - 1.0).abs() < 10.0 * Self::DOUBLE_EPSILON
    }

    /// Returns whether the float is "close" to 1.
    #[inline]
    pub fn is_one_f32(value: f32) -> bool {
        (value - 1.0).abs() < 10.0 * Self::FLOAT_EPSILON
    }

    /// Returns whether the double is "close" to 0. Same as `are_close(value, 0.0)` but faster.
    #[inline]
    pub fn is_zero(value: f64) -> bool {
        value.abs() < 10.0 * Self::DOUBLE_EPSILON
    }

    /// Returns whether the float is "close" to 0.
    #[inline]
    pub fn is_zero_f32(value: f32) -> bool {
        value.abs() < 10.0 * Self::FLOAT_EPSILON
    }

    /// Clamps a value between a minimum and maximum value. NaN is returned unchanged.
    ///
    /// # Panics
    /// Panics if `min > max` (a programmer error).
    #[inline]
    pub fn clamp(val: f64, min: f64, max: f64) -> f64 {
        if min > max {
            throw_cannot_be_greater_than(min, max);
        }

        if val < min {
            min
        } else if val > max {
            max
        } else {
            val
        }
    }

    /// Clamps a value between a minimum and maximum value. The bounds may be given
    /// in either order.
    #[inline]
    pub fn clamp_f32(value: f32, min: f32, max: f32) -> f32 {
        let amax = max_f32(min, max);
        let amin = min_f32(min, max);
        min_f32(max_f32(value, amin), amax)
    }

    /// Clamps a value between a minimum and maximum value.
    ///
    /// # Panics
    /// Panics if `min > max` (a programmer error).
    #[inline]
    pub fn clamp_i32(val: i32, min: i32, max: i32) -> i32 {
        if min > max {
            throw_cannot_be_greater_than(min, max);
        }

        if val < min {
            min
        } else if val > max {
            max
        } else {
            val
        }
    }

    /// Converts an angle in degrees to radians.
    #[inline]
    pub fn deg2rad(angle: f64) -> f64 {
        angle * (std::f64::consts::PI / 180.0)
    }

    /// Converts an angle in gradians to radians.
    #[inline]
    pub fn grad2rad(angle: f64) -> f64 {
        angle * (std::f64::consts::PI / 200.0)
    }

    /// Converts an angle in turns to radians.
    #[inline]
    pub fn turn2rad(angle: f64) -> f64 {
        angle * 2.0 * std::f64::consts::PI
    }

    /// Calculates the point of an angle (in radians) on an ellipse.
    #[inline]
    pub fn get_ellipse_point(centre: Point, radius_x: f64, radius_y: f64, angle: f64) -> Point {
        Point::new(
            radius_x * angle.cos() + centre.x,
            radius_y * angle.sin() + centre.y,
        )
    }

    /// Gets the minimum and maximum from the specified numbers.
    #[inline]
    pub fn get_min_max(a: f64, b: f64) -> (f64, f64) {
        if a < b {
            (a, b)
        } else {
            (b, a)
        }
    }

    /// Gets the minimum and maximum from the specified number and the difference with that number.
    #[inline]
    pub fn get_min_max_from_delta(initial_value: f64, delta: f64) -> (f64, f64) {
        Self::get_min_max(initial_value, initial_value + delta)
    }

    /// True for negative values (including `-0.0`), infinities and NaN.
    #[inline]
    pub fn is_negative_or_non_finite(d: f64) -> bool {
        d.to_bits() >= 0x7FF0_0000_0000_0000
    }

    #[inline]
    pub fn is_finite(d: f64) -> bool {
        d.is_finite()
    }

    /// Classifies on which side of an edge a polygon lies: `1` when all of its
    /// first `c_poly` points are on the positive side, `-1` when all are on the
    /// negative side, `0` when it straddles (or touches) the edge.
    pub fn which_polygon_side_intersects(
        c_poly: u32,
        p_pt_poly: &[Vector],
        pt_current: Vector,
        vec_edge: Vector,
    ) -> i32 {
        let mut n_positive = 0u32;
        let mut n_negative = 0u32;
        let mut n_zero = 0u32;

        let vec_edge_normal = Vector::new(-vec_edge.y, vec_edge.x);

        for pt in &p_pt_poly[..c_poly as usize] {
            let vec_current = pt_current - *pt;
            let r_dot = Vector::dot(vec_current, vec_edge_normal);

            if r_dot > 0.0 {
                n_positive += 1;
            } else if r_dot < 0.0 {
                n_negative += 1;
            } else {
                n_zero += 1;
            }

            if (n_positive > 0 && n_negative > 0) || (n_zero > 0) {
                return 0;
            }
        }

        if n_positive > 0 {
            1
        } else {
            -1
        }
    }

    /// Separating-axis test for two convex polygons.
    pub fn do_polygons_intersect(
        c_poly_a: u32,
        p_pt_poly_a: &[Vector],
        c_poly_b: u32,
        p_pt_poly_b: &[Vector],
    ) -> bool {
        let (na, nb) = (c_poly_a as usize, c_poly_b as usize);

        for i in 0..na {
            let vec_edge = p_pt_poly_a[(i + 1) % na] - p_pt_poly_a[i];
            if Self::which_polygon_side_intersects(c_poly_b, p_pt_poly_b, p_pt_poly_a[i], vec_edge)
                < 0
            {
                return false;
            }
        }

        for i in 0..nb {
            let vec_edge = p_pt_poly_b[(i + 1) % nb] - p_pt_poly_b[i];
            if Self::which_polygon_side_intersects(c_poly_a, p_pt_poly_a, p_pt_poly_b[i], vec_edge)
                < 0
            {
                return false;
            }
        }

        true
    }

    /// Returns whether polygon A lies entirely inside convex polygon B.
    pub fn is_entirely_contained(
        c_poly_a: u32,
        p_pt_poly_a: &[Vector],
        c_poly_b: u32,
        p_pt_poly_b: &[Vector],
    ) -> bool {
        let nb = c_poly_b as usize;

        for i in 0..nb {
            let vec_edge = p_pt_poly_b[(i + 1) % nb] - p_pt_poly_b[i];
            if Self::which_polygon_side_intersects(c_poly_a, p_pt_poly_a, p_pt_poly_b[i], vec_edge)
                <= 0
            {
                // The whole of the polygon is entirely on the outside of the edge,
                // so we can never intersect
                return false;
            }
        }

        true
    }
}

#[cold]
#[inline(never)]
fn throw_cannot_be_greater_than<T: std::fmt::Display>(min: T, max: T) -> ! {
    panic!("{min} cannot be greater than {max}.");
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANY_VALUE: f64 = 42.42;

    struct Fixture {
        calculated_any_value: f64,
        one: f64,
        zero: f64,
    }

    fn fixture() -> Fixture {
        let mut calculated_any_value = 0.0;
        let mut one = 0.0;
        let mut zero = 1.0;

        const N: i32 = 10;
        let dx_any = ANY_VALUE / N as f64;
        let dx_one = 1.0 / N as f64;
        let dx_zero = zero / N as f64;

        for _ in 0..N {
            calculated_any_value += dx_any;
            one += dx_one;
            zero -= dx_zero;
        }

        Fixture {
            calculated_any_value,
            one,
            zero,
        }
    }

    /// Rounds to `digits` decimals, ties to even.
    fn round(value: f64, digits: i32) -> f64 {
        let power = 10f64.powi(digits);
        (value * power).round_ties_even() / power
    }

    #[test]
    fn two_equivalent_double_values_are_close() {
        let f = fixture();
        let actual = MathUtilities::are_close(ANY_VALUE, f.calculated_any_value);

        assert!(actual);
        assert_eq!(ANY_VALUE, round(f.calculated_any_value, 14));
    }

    #[test]
    fn two_equivalent_single_values_are_close() {
        let f = fixture();
        let expected_value = ANY_VALUE as f32;
        let actual_value = f.calculated_any_value as f32;

        let actual = MathUtilities::are_close_f32(expected_value, actual_value);

        assert!(actual);
        assert_eq!(
            round(expected_value as f64, 5) as f32,
            round(actual_value as f64, 4) as f32
        );
    }

    #[test]
    fn calculated_double_one_is_one() {
        let f = fixture();
        let actual = MathUtilities::is_one(f.one);

        assert!(actual);
        assert_eq!(1.0, round(f.one, 15));
    }

    #[test]
    fn calculated_single_one_is_one() {
        let f = fixture();
        let actual_value = f.one as f32;

        let actual = MathUtilities::is_one_f32(actual_value);

        assert!(actual);
        assert_eq!(1.0f32, round(actual_value as f64, 7) as f32);
    }

    #[test]
    fn calculated_double_zero_is_zero() {
        let f = fixture();
        let actual = MathUtilities::is_zero(f.zero);

        assert!(actual);
        assert_eq!(0.0, round(f.zero, 15));
    }

    #[test]
    fn calculated_single_zero_is_zero() {
        let f = fixture();
        let actual_value = f.zero as f32;

        let actual = MathUtilities::is_zero_f32(actual_value);

        assert!(actual);
        assert_eq!(0.0f32, round(actual_value as f64, 7) as f32);
    }

    #[test]
    fn float_clamp_input_nan_return_nan() {
        let clamp = MathUtilities::clamp(f64::NAN, 0.0, 1.0);
        assert!(clamp.is_nan());
    }

    #[test]
    fn float_clamp_input_negative_infinity_return_min() {
        const MIN: f64 = 0.0;
        const MAX: f64 = 1.0;

        let actual = MathUtilities::clamp(f64::NEG_INFINITY, MIN, MAX);
        assert_eq!(MIN, actual);
    }

    #[test]
    fn float_clamp_input_positive_infinity_return_max() {
        const MIN: f64 = 0.0;
        const MAX: f64 = 1.0;

        let actual = MathUtilities::clamp(f64::INFINITY, MIN, MAX);
        assert_eq!(MAX, actual);
    }

    #[test]
    fn double_float_zero_less_than_one() {
        assert!(MathUtilities::less_than(0.0, 1.0));
    }

    #[test]
    fn single_float_zero_less_than_one() {
        assert!(MathUtilities::less_than_f32(0.0, 1.0));
    }

    #[test]
    fn double_float_one_not_less_than_zero() {
        assert!(!MathUtilities::less_than(1.0, 0.0));
    }

    #[test]
    fn single_float_one_not_less_than_zero() {
        assert!(!MathUtilities::less_than_f32(1.0, 0.0));
    }

    #[test]
    fn double_float_zero_not_greater_than_one() {
        assert!(!MathUtilities::greater_than(0.0, 1.0));
    }

    #[test]
    fn single_float_zero_not_greater_than_one() {
        assert!(!MathUtilities::greater_than_f32(0.0, 1.0));
    }

    #[test]
    fn double_float_one_greater_than_zero() {
        assert!(MathUtilities::greater_than(1.0, 0.0));
    }

    #[test]
    fn single_float_one_greater_than_zero() {
        assert!(MathUtilities::greater_than_f32(1.0, 0.0));
    }

    #[test]
    fn double_float_one_less_than_or_close_one() {
        assert!(MathUtilities::less_than_or_close(1.0, 1.0));
    }

    #[test]
    fn single_float_one_less_than_or_close_one() {
        assert!(MathUtilities::less_than_or_close_f32(1.0, 1.0));
    }

    #[test]
    fn double_float_one_greater_than_or_close_one() {
        assert!(MathUtilities::greater_than_or_close(1.0, 1.0));
    }

    #[test]
    fn single_float_one_greater_than_or_close_one() {
        assert!(MathUtilities::greater_than_or_close_f32(1.0, 1.0));
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn min_max_propagate_nan_and_order_zeros() {
        assert!(min(f64::NAN, 1.0).is_nan());
        assert!(min(1.0, f64::NAN).is_nan());
        assert!(max(f64::NAN, 1.0).is_nan());
        assert!(max(1.0, f64::NAN).is_nan());
        assert!(min(0.0, -0.0).is_sign_negative());
        assert!(max(-0.0, 0.0).is_sign_positive());
        assert_eq!(min(1.0, 2.0), 1.0);
        assert_eq!(max(1.0, 2.0), 2.0);
    }

    #[test]
    fn negative_or_non_finite() {
        assert!(MathUtilities::is_negative_or_non_finite(-1.0));
        assert!(MathUtilities::is_negative_or_non_finite(-0.0));
        assert!(MathUtilities::is_negative_or_non_finite(f64::NAN));
        assert!(MathUtilities::is_negative_or_non_finite(f64::INFINITY));
        assert!(!MathUtilities::is_negative_or_non_finite(0.0));
        assert!(!MathUtilities::is_negative_or_non_finite(1.0));
    }

    #[test]
    fn polygon_intersection() {
        let square = |x: f64, y: f64, s: f64| {
            [
                Vector::new(x, y),
                Vector::new(x, y + s),
                Vector::new(x + s, y + s),
                Vector::new(x + s, y),
            ]
        };
        let a = square(0.0, 0.0, 10.0);
        let b = square(5.0, 5.0, 10.0);
        let c = square(20.0, 20.0, 5.0);
        let d = square(2.0, 2.0, 2.0);

        assert!(MathUtilities::do_polygons_intersect(4, &a, 4, &b));
        assert!(!MathUtilities::do_polygons_intersect(4, &a, 4, &c));
        assert!(MathUtilities::is_entirely_contained(4, &d, 4, &a));
        assert!(!MathUtilities::is_entirely_contained(4, &b, 4, &a));
    }
}
