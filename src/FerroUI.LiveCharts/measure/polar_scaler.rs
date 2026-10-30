use super::ScaleError;
use ferroui_base::{Point, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolarScaleOptions {
    pub min_angle: f64,
    pub max_angle: f64,
    pub min_radius: f64,
    pub max_radius: f64,
    pub inner_radius: f64,
    pub initial_rotation: f64,
    pub total_angle: f64,
    pub angle_inverted: bool,
    pub radius_inverted: bool,
}

impl Default for PolarScaleOptions {
    fn default() -> Self {
        Self {
            min_angle: 0.0,
            max_angle: 360.0,
            min_radius: 0.0,
            max_radius: 1.0,
            inner_radius: 0.0,
            initial_rotation: 0.0,
            total_angle: 360.0,
            angle_inverted: false,
            radius_inverted: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolarScaler {
    pub center: Point,
    pub min_radius: f64,
    pub max_radius: f64,
    pub min_angle: f64,
    pub max_angle: f64,
    pub inner_radius: f64,
    scalable_radius: f64,
    initial_rotation: f64,
    total_angle: f64,
}

impl PolarScaler {
    pub fn new(plot: Rect, options: PolarScaleOptions) -> Result<Self, ScaleError> {
        if ![
            plot.x,
            plot.y,
            plot.width,
            plot.height,
            options.min_angle,
            options.max_angle,
            options.min_radius,
            options.max_radius,
            options.inner_radius,
            options.initial_rotation,
            options.total_angle,
        ]
        .iter()
        .all(|v| v.is_finite())
        {
            return Err(ScaleError::NonFiniteInput);
        }
        let scalable_radius = plot.width.min(plot.height) * 0.5 - options.inner_radius;
        if scalable_radius <= 0.0
            || options.inner_radius < 0.0
            || options.min_angle == options.max_angle
            || options.min_radius == options.max_radius
            || options.total_angle == 0.0
        {
            return Err(ScaleError::CollapsedPolarRange);
        }
        let (min_angle, max_angle) = if options.angle_inverted {
            (options.max_angle, options.min_angle)
        } else {
            (options.min_angle, options.max_angle)
        };
        let (min_radius, max_radius) = if options.radius_inverted {
            (options.max_radius, options.min_radius)
        } else {
            (options.min_radius, options.max_radius)
        };
        Ok(Self {
            center: Point::new(plot.x + plot.width * 0.5, plot.y + plot.height * 0.5),
            min_radius,
            max_radius,
            min_angle,
            max_angle,
            inner_radius: options.inner_radius,
            scalable_radius,
            initial_rotation: options.initial_rotation,
            total_angle: options.total_angle,
        })
    }

    /// Source-compatible angle convention: angle values are measured from zero;
    /// the bounds set the angular interval length, not its angular origin.
    pub fn get_angle(self, angle: f64) -> f64 {
        self.initial_rotation + self.total_angle * angle / (self.max_angle - self.min_angle)
    }
    pub fn to_pixels(self, angle: f64, radius: f64) -> Point {
        self.to_pixels_with_angle_in_degrees(self.get_angle(angle), radius)
    }
    pub fn to_pixels_with_angle_in_degrees(self, angle: f64, radius: f64) -> Point {
        let radius = self.inner_radius
            + self.scalable_radius * (radius - self.min_radius)
                / (self.max_radius - self.min_radius);
        let angle = angle.to_radians();
        Point::new(
            self.center.x + angle.cos() * radius,
            self.center.y + angle.sin() * radius,
        )
    }

    /// Inverse of `to_pixels`, using `atan2` so axes and every quadrant work.
    /// Values are normalized to one revolution starting at the initial rotation.
    /// This corrects the upstream inverse's division by zero and extra MinAngle.
    pub fn to_chart_values(self, point: Point) -> Point {
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        let radius = self.min_radius
            + (dx.hypot(dy) - self.inner_radius) / self.scalable_radius
                * (self.max_radius - self.min_radius);
        let relative_angle = dy.atan2(dx).to_degrees() - self.initial_rotation;
        let angle = if self.total_angle / (self.max_angle - self.min_angle) >= 0.0 {
            relative_angle.rem_euclid(360.0)
        } else {
            -(-relative_angle).rem_euclid(360.0)
        };
        Point::new(
            angle * (self.max_angle - self.min_angle) / self.total_angle,
            radius,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn polar_axis_quadrants_rotation_and_hole_roundtrip() {
        let s = PolarScaler::new(
            Rect::new(10.0, 20.0, 300.0, 200.0),
            PolarScaleOptions {
                min_radius: 10.0,
                max_radius: 30.0,
                inner_radius: 20.0,
                initial_rotation: 30.0,
                ..PolarScaleOptions::default()
            },
        )
        .unwrap();
        for angle in [0.0, 30.0, 90.0, 180.0, 270.0, 359.0] {
            let p = s.to_chart_values(s.to_pixels(angle, 20.0));
            assert!((p.x - angle).abs() < 1e-9 || (p.x - angle - 360.0).abs() < 1e-9);
            assert!((p.y - 20.0).abs() < 1e-9);
        }
    }
    #[test]
    fn nonzero_angle_bounds_keep_source_forward_convention() {
        let s = PolarScaler::new(
            Rect::new(0.0, 0.0, 200.0, 200.0),
            PolarScaleOptions {
                min_angle: 10.0,
                max_angle: 370.0,
                ..PolarScaleOptions::default()
            },
        )
        .unwrap();
        assert_eq!(s.get_angle(90.0), 90.0);
        assert!((s.to_chart_values(s.to_pixels(90.0, 1.0)).x - 90.0).abs() < 1e-9);
    }
    #[test]
    fn collapsed_polar_range_returns_error() {
        assert!(
            PolarScaler::new(Rect::new(0.0, 0.0, 0.0, 0.0), PolarScaleOptions::default()).is_err()
        );
    }
    #[test]
    fn inverted_angle_and_radius_roundtrip_without_changing_revolution() {
        let s = PolarScaler::new(
            Rect::new(0.0, 0.0, 200.0, 200.0),
            PolarScaleOptions {
                min_radius: 0.0,
                max_radius: 10.0,
                angle_inverted: true,
                radius_inverted: true,
                ..PolarScaleOptions::default()
            },
        )
        .unwrap();
        for angle in [30.0, 90.0, 180.0, 270.0, 359.0] {
            let p = s.to_chart_values(s.to_pixels(angle, 3.0));
            assert!((p.x - angle).abs() < 1e-9);
            assert!((p.y - 3.0).abs() < 1e-9);
        }
    }
}
