//! `Utils/DoubleUtils.cs`: the comparisons of the column layout (the double path of the
//! framework's math utilities).

// The smallest value such that 1.0 + DOUBLE_EPSILON != 1.0.
const DOUBLE_EPSILON: f64 = 2.2204460492503131e-016;

/// Whether `value` is zero within the epsilon of the layout.
pub(crate) fn is_zero(value: f64) -> bool {
    value.abs() < 10.0 * DOUBLE_EPSILON
}

/// Whether two values are equal within the epsilon of the layout.
pub(crate) fn are_close(value1: f64, value2: f64) -> bool {
    // In case they are infinities (then the epsilon check does not work).
    if value1 == value2 {
        return true;
    }

    let eps = (value1.abs() + value2.abs() + 10.0) * DOUBLE_EPSILON;
    let delta = value1 - value2;
    (-eps < delta) && (eps > delta)
}

/// Whether `value1` is greater than `value2` and not close to it.
pub(crate) fn greater_than(value1: f64, value2: f64) -> bool {
    (value1 > value2) && !are_close(value1, value2)
}

/// Whether two widths are the same: equal (`double.Equals`, under which NaN equals NaN) or close.
pub(crate) fn widths_equal(x: f64, y: f64) -> bool {
    (x.is_nan() && y.is_nan()) || x == y || are_close(x, y)
}

/// Whether two values are equal as `EqualityComparer<double>.Default` compares them (NaN equals
/// NaN), the test of `RaiseAndSetIfChanged`.
pub(crate) fn double_equals(x: f64, y: f64) -> bool {
    x == y || (x.is_nan() && y.is_nan())
}
