use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EasingError {
    InvalidControlPoint,
    TooFewKeyFrames,
    InvalidKeyFrameTime,
}
impl fmt::Display for EasingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidControlPoint => {
                "Bezier control points must be finite and X values in [0, 1]."
            }
            Self::TooFewKeyFrames => "At least two key frames are required.",
            Self::InvalidKeyFrameTime => {
                "Key frame times must be strictly increasing and finite within [0, 1]."
            }
        })
    }
}
impl std::error::Error for EasingError {}

#[derive(Clone, Debug)]
pub struct CubicBezierEasingFunction {
    samples: Option<Box<[f64; 257]>>,
}

impl CubicBezierEasingFunction {
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Result<Self, EasingError> {
        if ![x1, y1, x2, y2].iter().all(|v| v.is_finite())
            || !(0.0..=1.0).contains(&x1)
            || !(0.0..=1.0).contains(&x2)
        {
            return Err(EasingError::InvalidControlPoint);
        }
        if x1 == y1 && x2 == y2 {
            return Ok(Self { samples: None });
        }
        let (ax, bx, cx) = (1.0 - 3.0 * x2 + 3.0 * x1, 3.0 * x2 - 6.0 * x1, 3.0 * x1);
        let (ay, by, cy) = (1.0 - 3.0 * y2 + 3.0 * y1, 3.0 * y2 - 6.0 * y1, 3.0 * y1);
        let mut samples = Box::new([0.0; 257]);
        for (i, sample) in samples.iter_mut().enumerate() {
            let x = i as f64 / 256.0;
            let (mut lo, mut hi, mut t) = (0.0, 1.0, x);
            for _ in 0..24 {
                let fx = ((ax * t + bx) * t + cx) * t - x;
                if fx.abs() < 1e-6 {
                    break;
                }
                if fx > 0.0 {
                    hi = t;
                } else {
                    lo = t;
                }
                let slope = (3.0 * ax * t + 2.0 * bx) * t + cx;
                let next = if slope.abs() < 1e-6 {
                    f64::NAN
                } else {
                    t - fx / slope
                };
                t = if next > lo && next < hi {
                    next
                } else {
                    0.5 * (lo + hi)
                };
            }
            *sample = ((ay * t + by) * t + cy) * t;
        }
        Ok(Self {
            samples: Some(samples),
        })
    }

    pub fn sample(&self, x: f64) -> f64 {
        let Some(samples) = &self.samples else {
            return x;
        };
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        if x.is_nan() {
            return f64::NAN;
        }
        let f = x * 256.0;
        let i = f as usize;
        samples[i] + (samples[i + 1] - samples[i]) * (f - i as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reference(x: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        let value = |t: f64, a: f64, b: f64| {
            (((1.0 - 3.0 * b + 3.0 * a) * t + (3.0 * b - 6.0 * a)) * t + 3.0 * a) * t
        };
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..80 {
            let mid = (lo + hi) / 2.0;
            if value(mid, x1, x2) < x {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        value((lo + hi) / 2.0, y1, y2)
    }
    #[test]
    fn lut_matches_analytic_across_domain() {
        for (x1, y1, x2, y2) in [
            (0.25, 0.1, 0.25, 1.0),
            (0.42, 0.0, 1.0, 1.0),
            (0.0, 0.0, 0.58, 1.0),
            (0.42, 0.0, 0.58, 1.0),
            (0.8, 0.0, 0.2, 1.0),
            (0.0, 0.6, 0.0, 1.0),
        ] {
            let curve = CubicBezierEasingFunction::new(x1, y1, x2, y2).unwrap();
            let mut previous = f64::NEG_INFINITY;
            for i in 0..=200 {
                let x = i as f64 / 200.0;
                let actual = curve.sample(x);
                let expected = reference(x, x1, y1, x2, y2);
                assert!(
                    (actual - expected).abs() < 5e-3,
                    "x={x}: actual={actual}, expected={expected}"
                );
                assert!(actual >= previous - 1e-4);
                previous = actual;
            }
        }
    }
    #[test]
    fn linear_shortcut_is_identity_curve() {
        let curve = CubicBezierEasingFunction::new(0.3, 0.3, 0.7, 0.7).unwrap();
        for i in -10..=20 {
            let x = i as f64 / 10.0;
            assert_eq!(curve.sample(x), x);
        }
    }
    #[test]
    fn invalid_points_and_nan_sample_are_safe() {
        assert!(CubicBezierEasingFunction::new(f64::NAN, 0.0, 0.5, 1.0).is_err());
        assert!(CubicBezierEasingFunction::new(-0.1, 0.0, 0.5, 1.0).is_err());
        assert!(CubicBezierEasingFunction::new(0.1, 0.0, 0.5, 1.0)
            .unwrap()
            .sample(f64::NAN)
            .is_nan());
    }
}
