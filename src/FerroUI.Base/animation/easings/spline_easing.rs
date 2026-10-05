use crate::animation::easings::IEasing;
use crate::animation::KeySpline;
use crate::Ref;

/// Eases a value using a user-defined cubic bezier curve. Good for custom
/// easing functions that are not possible with the built-in ones.
pub struct SplineEasing {
    internal_key_spline: Ref<KeySpline>,
}

impl SplineEasing {
    /// Creates an easing with the default control points `(0, 0)` and
    /// `(1, 1)`, which is a linear easing.
    pub fn new() -> Self {
        Self { internal_key_spline: KeySpline::new() }
    }

    /// Creates an easing with the given control points. Panics when an X
    /// coordinate is outside `[0, 1]`.
    pub fn with_points(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let result = Self::new();
        result.set_x1(x1);
        result.set_y1(y1);
        result.set_x2(x2);
        result.set_y2(y2);
        result
    }

    /// Creates an easing that uses `key_spline`.
    pub fn with_key_spline(key_spline: Ref<KeySpline>) -> Self {
        Self { internal_key_spline: key_spline }
    }

    /// X coordinate of the first control point.
    pub fn x1(&self) -> f64 {
        self.internal_key_spline.control_point_x1()
    }

    pub fn set_x1(&self, value: f64) {
        self.internal_key_spline.set_control_point_x1(value)
    }

    /// Y coordinate of the first control point.
    pub fn y1(&self) -> f64 {
        self.internal_key_spline.control_point_y1()
    }

    pub fn set_y1(&self, value: f64) {
        self.internal_key_spline.set_control_point_y1(value)
    }

    /// X coordinate of the second control point.
    pub fn x2(&self) -> f64 {
        self.internal_key_spline.control_point_x2()
    }

    pub fn set_x2(&self, value: f64) {
        self.internal_key_spline.set_control_point_x2(value)
    }

    /// Y coordinate of the second control point.
    pub fn y2(&self) -> f64 {
        self.internal_key_spline.control_point_y2()
    }

    pub fn set_y2(&self, value: f64) {
        self.internal_key_spline.set_control_point_y2(value)
    }
}

impl Default for SplineEasing {
    fn default() -> Self {
        Self::new()
    }
}

impl IEasing for SplineEasing {
    fn ease(&self, progress: f64) -> f64 {
        self.internal_key_spline.get_spline_progress(progress)
    }
}
