// Ported from WPF open-source code.
// https://github.com/dotnet/wpf/blob/ae1790531c3b993b56eba8b1f0dd395a3ed7de75/src/Microsoft.DotNet.Wpf/src/PresentationCore/System/Windows/Media/Animation/KeySpline.cs

use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Ref};
use std::cell::Cell;

/// 1/3 the desired accuracy in X.
const ACCURACY: f64 = 0.001;
/// Computational zero.
const FUZZ: f64 = 0.000001;

/// Determines how an animation is used based on a cubic bezier curve. X1
/// and X2 must be between 0.0 and 1.0, inclusive.
#[repr(C)]
pub struct KeySpline {
    base: FerroObject,

    // Control points
    control_point_x1: Cell<f64>,
    control_point_y1: Cell<f64>,
    control_point_x2: Cell<f64>,
    control_point_y2: Cell<f64>,
    is_specified: Cell<bool>,
    is_dirty: Cell<bool>,

    // The parameter that corresponds to the most recent time
    parameter: Cell<f64>,

    // Cached coefficients
    bx: Cell<f64>,       // 3*points[0].X
    cx: Cell<f64>,       // 3*points[1].X
    cx_bx: Cell<f64>,    // 2*(Cx - Bx)
    three_cx: Cell<f64>, // 3 - Cx

    by: Cell<f64>, // 3*points[0].Y
    cy: Cell<f64>, // 3*points[1].Y
}

ferro_class!(KeySpline: FerroObject);
crate::ferro_class_info!(KeySpline { new: KeySpline::new });
ferro_impl_classes!(KeySpline: FerroObjectImpl);

impl KeySpline {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self {
            base: FerroObject::construct(),
            control_point_x1: Cell::new(x1),
            control_point_y1: Cell::new(y1),
            control_point_x2: Cell::new(x2),
            control_point_y2: Cell::new(y2),
            is_specified: Cell::new(false),
            is_dirty: Cell::new(true),
            parameter: Cell::new(0.0),
            bx: Cell::new(0.0),
            cx: Cell::new(0.0),
            cx_bx: Cell::new(0.0),
            three_cx: Cell::new(0.0),
            by: Cell::new(0.0),
            cy: Cell::new(0.0),
        }
    }

    /// Creates a key spline with the default control points `(0, 0)` and
    /// `(1, 1)`.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct(0.0, 0.0, 1.0, 1.0))
    }

    /// Creates a key spline with the given control points. The points are
    /// not validated here; see [`is_valid`](Self::is_valid).
    pub fn with_points(x1: f64, y1: f64, x2: f64, y2: f64) -> Ref<Self> {
        instantiate(Self::construct(x1, y1, x2, y2))
    }

    /// Parses a key spline from four numbers.
    pub fn parse(value: &str) -> Result<Ref<KeySpline>, FormatError> {
        let error = |_| FormatError::from_string(format!("Invalid KeySpline string: \"{value}\"."));
        let mut tokenizer = SpanStringTokenizer::new(value);
        let x1 = tokenizer.read_double().map_err(error)?;
        let y1 = tokenizer.read_double().map_err(error)?;
        let x2 = tokenizer.read_double().map_err(error)?;
        let y2 = tokenizer.read_double().map_err(error)?;
        tokenizer.finish().map_err(error)?;
        Ok(KeySpline::with_points(x1, y1, x2, y2))
    }

    /// X coordinate of the first control point.
    pub fn control_point_x1(&self) -> f64 {
        self.control_point_x1.get()
    }

    /// Sets the X coordinate of the first control point. Panics when the
    /// value is outside `[0, 1]`.
    pub fn set_control_point_x1(&self, value: f64) {
        if Self::is_valid_x_value(value) {
            self.control_point_x1.set(value);
            self.is_dirty.set(true);
        } else {
            panic!("Invalid KeySpline X1 value. Must be >= 0.0 and <= 1.0.");
        }
    }

    /// Y coordinate of the first control point.
    pub fn control_point_y1(&self) -> f64 {
        self.control_point_y1.get()
    }

    pub fn set_control_point_y1(&self, value: f64) {
        self.control_point_y1.set(value);
        self.is_dirty.set(true);
    }

    /// X coordinate of the second control point.
    pub fn control_point_x2(&self) -> f64 {
        self.control_point_x2.get()
    }

    /// Sets the X coordinate of the second control point. Panics when the
    /// value is outside `[0, 1]`.
    pub fn set_control_point_x2(&self, value: f64) {
        if Self::is_valid_x_value(value) {
            self.control_point_x2.set(value);
            self.is_dirty.set(true);
        } else {
            panic!("Invalid KeySpline X2 value. Must be >= 0.0 and <= 1.0.");
        }
    }

    /// Y coordinate of the second control point.
    pub fn control_point_y2(&self) -> f64 {
        self.control_point_y2.get()
    }

    pub fn set_control_point_y2(&self, value: f64) {
        self.control_point_y2.set(value);
        self.is_dirty.set(true);
    }

    /// Calculates the spline progress from a linear progress.
    pub fn get_spline_progress(&self, linear_progress: f64) -> f64 {
        if self.is_dirty.get() {
            self.build();
        }

        if !self.is_specified.get() {
            linear_progress
        } else {
            self.set_parameter_from_x(linear_progress);

            Self::get_bezier_value(self.by.get(), self.cy.get(), self.parameter.get())
        }
    }

    /// Checks whether the spline control points are valid: both X values
    /// within `[0, 1]`.
    pub fn is_valid(&self) -> bool {
        Self::is_valid_x_value(self.control_point_x1.get()) && Self::is_valid_x_value(self.control_point_x2.get())
    }

    fn is_valid_x_value(value: f64) -> bool {
        (0.0..=1.0).contains(&value)
    }

    /// Computes the cached coefficients.
    fn build(&self) {
        if self.control_point_x1.get() == 0.0
            && self.control_point_y1.get() == 0.0
            && self.control_point_x2.get() == 1.0
            && self.control_point_y2.get() == 1.0
        {
            // This KeySpline would have no effect on the progress.
            self.is_specified.set(false);
        } else {
            self.is_specified.set(true);

            self.parameter.set(0.0);

            // X coefficients
            self.bx.set(3.0 * self.control_point_x1.get());
            self.cx.set(3.0 * self.control_point_x2.get());
            self.cx_bx.set(2.0 * (self.cx.get() - self.bx.get()));
            self.three_cx.set(3.0 - self.cx.get());

            // Y coefficients
            self.by.set(3.0 * self.control_point_y1.get());
            self.cy.set(3.0 * self.control_point_y2.get());
        }

        self.is_dirty.set(false);
    }

    /// Gets a bezier value: `b*t*(1-t)^2 + c*t^2*(1-t) + t^3`.
    fn get_bezier_value(b: f64, c: f64, t: f64) -> f64 {
        let s = 1.0 - t;
        let t2 = t * t;

        b * t * s * s + c * t2 * s + t2 * t
    }

    /// Gets X and dX/dt at the parameter `t`.
    fn get_x_and_dx(&self, t: f64) -> (f64, f64) {
        let s = 1.0 - t;
        let t2 = t * t;
        let s2 = s * s;

        let x = self.bx.get() * t * s2 + self.cx.get() * t2 * s + t2 * t;
        let dx = self.bx.get() * s2 + self.cx_bx.get() * s * t + self.three_cx.get() * t2;
        (x, dx)
    }

    /// Computes the spline parameter from the X coordinate.
    fn set_parameter_from_x(&self, time: f64) {
        // Dynamic search interval to clamp with
        let mut bottom = 0.0;
        let mut top = 1.0;

        if time == 0.0 {
            self.parameter.set(0.0);
        } else if time == 1.0 {
            self.parameter.set(1.0);
        } else {
            let mut parameter = self.parameter.get();

            // Loop while improving the guess
            while top - bottom > FUZZ {
                // Get x and dx/dt at the current parameter
                let (x, dx) = self.get_x_and_dx(parameter);
                let absdx = dx.abs();

                // Clamp down the search interval, relying on the monotonicity of X(t)
                if x > time {
                    top = parameter; // because parameter > solution
                } else {
                    bottom = parameter; // because parameter < solution
                }

                // The desired accuracy is in ultimately in y, not in x, so the
                // accuracy needs to be multiplied by dx/dy = (dx/dt) / (dy/dt).
                // But dy/dt <=3, so we omit that
                if (x - time).abs() < ACCURACY * absdx {
                    break; // We're there
                }

                if absdx > FUZZ {
                    // Nonzero derivative, use Newton-Raphson to obtain the next guess
                    let next = parameter - (x - time) / dx;

                    // If next guess is out of the search interval then clamp it in
                    if next >= top {
                        parameter = (parameter + top) / 2.0;
                    } else if next <= bottom {
                        parameter = (parameter + bottom) / 2.0;
                    } else {
                        // Next guess is inside the search interval, accept it
                        parameter = next;
                    }
                } else {
                    // Zero derivative, halve the search interval
                    parameter = (bottom + top) / 2.0;
                }
            }

            self.parameter.set(parameter);
        }
    }
}

/// The curve of a [`KeySpline`] with fixed control points, as a plain value.
///
/// A key spline is an object of the UI thread; a spline easing that a key
/// frame animation of the compositor uses is evaluated on the render thread
/// with this solver. It is the same algorithm, with the same search state
/// (the parameter of the most recent progress is the next first guess).
#[derive(Clone, Copy, Debug)]
pub struct KeySplineSolver {
    is_specified: bool,
    parameter: f64,
    bx: f64,
    cx: f64,
    cx_bx: f64,
    three_cx: f64,
    by: f64,
    cy: f64,
}

impl KeySplineSolver {
    /// Creates the solver of the curve with the given control points.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let bx = 3.0 * x1;
        let cx = 3.0 * x2;
        Self {
            // This curve would have no effect on the progress.
            is_specified: !(x1 == 0.0 && y1 == 0.0 && x2 == 1.0 && y2 == 1.0),
            parameter: 0.0,
            bx,
            cx,
            cx_bx: 2.0 * (cx - bx),
            three_cx: 3.0 - cx,
            by: 3.0 * y1,
            cy: 3.0 * y2,
        }
    }

    /// Calculates the spline progress from a linear progress.
    pub fn get_spline_progress(&mut self, linear_progress: f64) -> f64 {
        if !self.is_specified {
            linear_progress
        } else {
            self.set_parameter_from_x(linear_progress);

            KeySpline::get_bezier_value(self.by, self.cy, self.parameter)
        }
    }

    fn get_x_and_dx(&self, t: f64) -> (f64, f64) {
        let s = 1.0 - t;
        let t2 = t * t;
        let s2 = s * s;

        let x = self.bx * t * s2 + self.cx * t2 * s + t2 * t;
        let dx = self.bx * s2 + self.cx_bx * s * t + self.three_cx * t2;
        (x, dx)
    }

    fn set_parameter_from_x(&mut self, time: f64) {
        let mut bottom = 0.0;
        let mut top = 1.0;

        if time == 0.0 {
            self.parameter = 0.0;
        } else if time == 1.0 {
            self.parameter = 1.0;
        } else {
            let mut parameter = self.parameter;

            while top - bottom > FUZZ {
                let (x, dx) = self.get_x_and_dx(parameter);
                let absdx = dx.abs();

                if x > time {
                    top = parameter;
                } else {
                    bottom = parameter;
                }

                if (x - time).abs() < ACCURACY * absdx {
                    break;
                }

                if absdx > FUZZ {
                    let next = parameter - (x - time) / dx;

                    if next >= top {
                        parameter = (parameter + top) / 2.0;
                    } else if next <= bottom {
                        parameter = (parameter + bottom) / 2.0;
                    } else {
                        parameter = next;
                    }
                } else {
                    parameter = (bottom + top) / 2.0;
                }
            }

            self.parameter = parameter;
        }
    }
}

#[cfg(test)]
mod solver_tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn the_solver_follows_the_key_spline() {
        for (x1, y1, x2, y2) in [(0.25, 0.1, 0.25, 1.0), (0.42, 0.0, 0.58, 1.0), (0.0, 0.0, 1.0, 1.0), (0.1, 0.9, 0.9, 0.1)]
        {
            let spline = KeySpline::with_points(x1, y1, x2, y2);
            let mut solver = KeySplineSolver::new(x1, y1, x2, y2);

            for step in 0..=100 {
                let progress = f64::from(step) / 100.0;
                assert_eq!(spline.get_spline_progress(progress), solver.get_spline_progress(progress));
            }
            // Backwards as well: the search starts from the last parameter.
            for step in (0..=100).rev() {
                let progress = f64::from(step) / 100.0;
                assert_eq!(spline.get_spline_progress(progress), solver.get_spline_progress(progress));
            }
        }
    }
}
