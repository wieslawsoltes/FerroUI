use super::AxisLimit;
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AxisOrientation {
    #[default]
    Unknown,
    X,
    Y,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleError {
    UnknownOrientation,
    NonFiniteInput,
    InvalidLogarithmicDomain,
    CollapsedPolarRange,
}

impl fmt::Display for ScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownOrientation => "The axis is not ready to be scaled.",
            Self::NonFiniteInput => "Scale inputs must be finite.",
            Self::InvalidLogarithmicDomain => {
                "A logarithmic scale requires positive limits and a finite base greater than one."
            }
            Self::CollapsedPolarRange => "Polar scale dimensions and ranges must be nonzero.",
        })
    }
}
impl std::error::Error for ScaleError {}

/// The source affine scaler, with an optional logarithmic value transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scaler {
    min_pixel: f64,
    min_value: f64,
    slope: f64,
    inverse_slope: f64,
    logarithm: Option<f64>,
}

impl Scaler {
    pub fn new(
        pixel_start: f64,
        pixel_size: f64,
        min: f64,
        max: f64,
        orientation: AxisOrientation,
        inverted: bool,
    ) -> Result<Self, ScaleError> {
        Self::with_min_step(
            pixel_start,
            pixel_size,
            min,
            max,
            orientation,
            inverted,
            0.0,
        )
    }

    pub fn with_min_step(
        pixel_start: f64,
        pixel_size: f64,
        mut min: f64,
        mut max: f64,
        orientation: AxisOrientation,
        inverted: bool,
        min_step: f64,
    ) -> Result<Self, ScaleError> {
        if orientation == AxisOrientation::Unknown {
            return Err(ScaleError::UnknownOrientation);
        }
        if ![pixel_start, pixel_size, min, max, min_step]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(ScaleError::NonFiniteInput);
        }
        AxisLimit::validate_limits(&mut min, &mut max, min_step);
        if (orientation == AxisOrientation::Y) ^ inverted {
            std::mem::swap(&mut min, &mut max);
        }
        let mut slope = pixel_size / (max - min);
        let mut inverse_slope = 1.0 / slope;
        if !slope.is_finite() || !inverse_slope.is_finite() {
            slope = 0.0;
            inverse_slope = 0.0;
        }
        Ok(Self {
            min_pixel: pixel_start,
            min_value: min,
            slope,
            inverse_slope,
            logarithm: None,
        })
    }

    pub fn logarithmic(
        pixel_start: f64,
        pixel_size: f64,
        min: f64,
        max: f64,
        orientation: AxisOrientation,
        inverted: bool,
        base: f64,
    ) -> Result<Self, ScaleError> {
        if min <= 0.0 || max <= 0.0 || base <= 1.0 || !base.is_finite() {
            return Err(ScaleError::InvalidLogarithmicDomain);
        }
        let mut result = Self::new(
            pixel_start,
            pixel_size,
            min.log(base),
            max.log(base),
            orientation,
            inverted,
        )?;
        result.logarithm = Some(base);
        Ok(result)
    }

    pub fn is_linear(self) -> bool {
        self.logarithm.is_none()
    }
    pub fn to_pixels(self, value: f64) -> f64 {
        let value = self.logarithm.map_or(value, |base| value.log(base));
        self.min_pixel + (value - self.min_value) * self.slope
    }
    pub fn to_chart_values(self, pixels: f64) -> f64 {
        let value = self.min_value + (pixels - self.min_pixel) * self.inverse_slope;
        self.logarithm.map_or(value, |base| base.powf(value))
    }
    pub fn measure_in_pixels(self, value: f64) -> f64 {
        (value * self.slope).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_x_y_inversion_roundtrips() {
        for orientation in [AxisOrientation::X, AxisOrientation::Y] {
            for inverted in [false, true] {
                let s = Scaler::new(20.0, 200.0, -5.0, 15.0, orientation, inverted).unwrap();
                for x in [-5.0, 0.0, 5.0, 15.0, 25.0] {
                    assert!((s.to_chart_values(s.to_pixels(x)) - x).abs() < 1e-12);
                }
                let reversed = (orientation == AxisOrientation::Y) ^ inverted;
                assert_eq!(s.to_pixels(-5.0), if reversed { 220.0 } else { 20.0 });
                assert_eq!(s.measure_in_pixels(2.0), 20.0);
            }
        }
    }

    #[test]
    fn invalid_and_collapsed_inputs_are_deterministic() {
        assert_eq!(
            Scaler::new(0.0, 10.0, 0.0, 1.0, AxisOrientation::Unknown, false),
            Err(ScaleError::UnknownOrientation)
        );
        let s = Scaler::new(20.0, 100.0, 5.0, 5.0, AxisOrientation::X, false).unwrap();
        assert_eq!(s.to_pixels(10.0), 20.0);
        assert_eq!(s.to_chart_values(50.0), 5.0);
        assert!(Scaler::new(0.0, 10.0, f64::NAN, 1.0, AxisOrientation::X, false).is_err());
    }

    #[test]
    fn logarithmic_scaler_preserves_value_ratios() {
        let s =
            Scaler::logarithmic(0.0, 300.0, 1.0, 1000.0, AxisOrientation::X, false, 10.0).unwrap();
        assert!(!s.is_linear());
        assert!((s.to_pixels(10.0) - 100.0).abs() < 1e-10);
        assert!((s.to_chart_values(200.0) - 100.0).abs() < 1e-10);
        assert!(
            Scaler::logarithmic(0.0, 300.0, 0.0, 1000.0, AxisOrientation::X, false, 10.0).is_err()
        );
    }
}
