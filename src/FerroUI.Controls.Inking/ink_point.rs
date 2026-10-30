use ferroui_base::Point;
use std::fmt;

/// Validation and persistence failures in the portable ink engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InkError {
    Platform(String),
    InvalidOperation(&'static str),
    InvalidInput(&'static str),
    InvalidPoint,
    InvalidDrawingAttributes,
    InvalidTransform,
    EmptyStroke,
    DuplicateStrokeId(u32),
    StrokeIdExhausted,
    InvalidData(&'static str),
    UnsupportedFormat,
    LimitExceeded,
}

impl fmt::Display for InkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(reason) => f.write_str(reason),
            Self::InvalidOperation(reason) | Self::InvalidInput(reason) => f.write_str(reason),
            Self::InvalidPoint => f.write_str(
                "Ink coordinates and device measurements must be finite and within range",
            ),
            Self::InvalidDrawingAttributes => f.write_str("Invalid ink drawing attributes"),
            Self::InvalidTransform => f.write_str("Ink transforms must be finite affine matrices"),
            Self::EmptyStroke => f.write_str("An ink stroke must contain at least one point"),
            Self::DuplicateStrokeId(id) => write!(f, "Duplicate ink stroke ID {id}"),
            Self::StrokeIdExhausted => f.write_str("Ink stroke IDs are exhausted"),
            Self::InvalidData(reason) => write!(f, "Invalid ink document: {reason}"),
            Self::UnsupportedFormat => f.write_str("Unsupported ink persistence format"),
            Self::LimitExceeded => {
                f.write_str("Ink document exceeds the configured resource limits")
            }
        }
    }
}

impl std::error::Error for InkError {}

/// An immutable device sample. Positions are in device-independent pixels,
/// pressure is normalized to 0..=1, angles are degrees and timestamps are
/// monotonically increasing microseconds supplied by the input adapter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InkPoint {
    pub position: Point,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub twist: f32,
    pub timestamp: u64,
}

impl InkPoint {
    pub fn new(position: Point, pressure: f32) -> Result<Self, InkError> {
        let point = Self {
            position,
            pressure,
            tilt_x: 0.0,
            tilt_y: 0.0,
            twist: 0.0,
            timestamp: 0,
        };
        point.validate()?;
        Ok(point)
    }

    pub fn validate(&self) -> Result<(), InkError> {
        if !valid_position(self.position)
            || !self.pressure.is_finite()
            || !(0.0..=1.0).contains(&self.pressure)
            || !self.tilt_x.is_finite()
            || !(-90.0..=90.0).contains(&self.tilt_x)
            || !self.tilt_y.is_finite()
            || !(-90.0..=90.0).contains(&self.tilt_y)
            || !self.twist.is_finite()
            || !(0.0..360.0).contains(&self.twist)
        {
            return Err(InkError::InvalidPoint);
        }
        Ok(())
    }

    pub(crate) fn interpolate(a: Self, b: Self, t: f64) -> Self {
        // Keep actual device samples bit-for-bit at segment boundaries. In
        // particular a floating-point timestamp interpolation cannot exactly
        // represent every u64, and f32 subtraction can lose small pressures.
        if t <= 0.0 {
            return a;
        }
        if t >= 1.0 {
            return b;
        }
        let mix = |a: f32, b: f32| (f64::from(a) + (f64::from(b) - f64::from(a)) * t) as f32;
        // Twist follows the shortest arc, including the 359 -> 0 boundary.
        let twist_delta =
            (f64::from(b.twist) - f64::from(a.twist) + 540.0).rem_euclid(360.0) - 180.0;
        let twist = (f64::from(a.twist) + twist_delta * t).rem_euclid(360.0) as f32;
        Self {
            position: Point::new(
                a.position.x + (b.position.x - a.position.x) * t,
                a.position.y + (b.position.y - a.position.y) * t,
            ),
            pressure: mix(a.pressure, b.pressure),
            tilt_x: mix(a.tilt_x, b.tilt_x),
            tilt_y: mix(a.tilt_y, b.tilt_y),
            // Rounding a value just below 360 to f32 can produce 360, which
            // is outside the public sample range and equivalent to zero.
            twist: if twist >= 360.0 { 0.0 } else { twist },
            timestamp: a
                .timestamp
                .saturating_add(((b.timestamp.saturating_sub(a.timestamp)) as f64 * t) as u64)
                .min(b.timestamp),
        }
    }
}

pub(crate) fn valid_position(value: Point) -> bool {
    // A finite working range also prevents intermediate geometry overflow.
    value.x.is_finite() && value.y.is_finite() && value.x.abs() <= 1.0e12 && value.y.abs() <= 1.0e12
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_device_measurements() {
        assert!(InkPoint::new(Point::new(f64::NAN, 0.0), 0.5).is_err());
        assert!(InkPoint::new(Point::default(), -0.1).is_err());
        assert!(InkPoint::new(Point::default(), 0.0).is_ok());
        let mut p = InkPoint::new(Point::default(), 0.5).unwrap();
        p.tilt_y = 91.0;
        assert!(p.validate().is_err());
    }
    #[test]
    fn twist_interpolates_across_zero() {
        let mut a = InkPoint::new(Point::default(), 0.5).unwrap();
        let mut b = a;
        a.twist = 359.0;
        b.twist = 1.0;
        assert_eq!(InkPoint::interpolate(a, b, 0.5).twist, 0.0);
    }
    #[test]
    fn interpolation_preserves_original_endpoints_and_large_timestamps() {
        let mut a = InkPoint::new(Point::new(1.0e12, 1.0e12), 1.0).unwrap();
        a.timestamp = 1;
        let mut b = InkPoint::new(Point::new(0.001, 0.001), 1.0e-8).unwrap();
        b.timestamp = (1u64 << 54) + 3;
        assert_eq!(InkPoint::interpolate(a, b, 0.0), a);
        assert_eq!(InkPoint::interpolate(a, b, 1.0), b);
        let near_end = InkPoint::interpolate(a, b, 1.0 - f64::EPSILON);
        assert!(near_end.timestamp <= b.timestamp);
        near_end.validate().unwrap();
    }
    #[test]
    fn twist_rounding_never_generates_an_invalid_360_degree_sample() {
        let a = InkPoint::new(Point::default(), 0.5).unwrap();
        let mut b = a;
        b.twist = 359.99;
        let sample = InkPoint::interpolate(a, b, 0.001);
        sample.validate().unwrap();
        assert_eq!(sample.twist, 0.0);
    }
}
