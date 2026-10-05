//! Velocity estimation for pointer movement, by least-squares fitting of
//! the most recent pointer positions.

use crate::Vector;
use std::time::Duration;

/// A velocity in two dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Velocity {
    pub pixels_per_second: Vector,
}

impl Velocity {
    pub fn new(pixels_per_second: Vector) -> Self {
        Self { pixels_per_second }
    }

    pub fn clamp_magnitude(self, min_value: f64, max_value: f64) -> Velocity {
        debug_assert!(min_value >= 0.0);
        debug_assert!(max_value >= 0.0 && max_value >= min_value);

        let value_squared = self.pixels_per_second.squared_length();

        // Preventing NaN in the result is important -- if a NaN eventually
        // gets into scroll gesture args it results in runtime errors.
        let scaled = |target: f64| {
            let length = self.pixels_per_second.length();
            Velocity::new(if length != 0.0 { (self.pixels_per_second / length) * target } else { Vector::default() })
        };

        if value_squared > max_value * max_value {
            return scaled(max_value);
        }

        if value_squared < min_value * min_value {
            return scaled(min_value);
        }

        self
    }
}

/// A two dimensional velocity estimate.
///
/// `confidence` is a measure of how well the velocity tracker's position
/// data fit a straight line, `duration` is the time that elapsed between
/// the first and last position sample used to compute the velocity, and
/// `offset` is similarly the difference between the first and last
/// positions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct VelocityEstimate {
    pub pixels_per_second: Vector,
    pub confidence: f64,
    pub duration: Duration,
    pub offset: Vector,
}

#[derive(Clone, Copy, Default)]
struct PointAtTime {
    valid: bool,
    point: Vector,
    time: Duration,
}

const ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS: f64 = 40.0;
const HISTORY_SIZE: usize = 20;
const HORIZON_MILLISECONDS: f64 = 100.0;
const MIN_SAMPLE_SIZE: usize = 3;
/// Logical pixels / second.
const MIN_FLING_VELOCITY: f64 = 50.0;
const MAX_FLING_VELOCITY: f64 = 8000.0;

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Computes a pointer's velocity based on data from pointer move events.
///
/// The input data is provided by calling `add_position`. Adding data is
/// cheap. To obtain a velocity, call `get_velocity` or
/// `get_velocity_estimate`. This will compute the velocity based on the
/// data added so far. Only call these when you need to use the velocity,
/// as they are comparatively expensive.
///
/// The quality of the velocity estimation will be better if more data
/// points have been received.
pub(crate) struct VelocityTracker {
    samples: [PointAtTime; HISTORY_SIZE],
    index: usize,
}

impl Default for VelocityTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl VelocityTracker {
    pub fn new() -> Self {
        Self { samples: [PointAtTime::default(); HISTORY_SIZE], index: 0 }
    }

    /// Adds a position as the given time to the tracker.
    pub fn add_position(&mut self, time: Duration, position: Vector) {
        self.index += 1;
        if self.index == HISTORY_SIZE {
            self.index = 0;
        }
        self.samples[self.index] = PointAtTime { valid: true, point: position, time };
    }

    /// Returns an estimate of the velocity of the object being tracked by
    /// the tracker given the current information available to the tracker.
    ///
    /// `now` is the time of the event the estimate is requested for, in the
    /// time base of the samples: when the last sample is older than the
    /// "pointer stopped" threshold the velocity is zero.
    ///
    /// Information is added using `add_position`. Returns `None` if there
    /// is no data on which to base an estimate.
    pub fn get_velocity_estimate(&self, now: Duration) -> Option<VelocityEstimate> {
        let mut index = self.index;
        let newest_sample = self.samples[index];

        if newest_sample.valid
            && millis(now.saturating_sub(newest_sample.time)) > ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS
        {
            return Some(VelocityEstimate {
                pixels_per_second: Vector::default(),
                confidence: 1.0,
                duration: Duration::ZERO,
                offset: Vector::default(),
            });
        }

        let mut x = [0.0; HISTORY_SIZE];
        let mut y = [0.0; HISTORY_SIZE];
        let mut w = [0.0; HISTORY_SIZE];
        let mut time = [0.0; HISTORY_SIZE];
        let mut sample_count = 0;

        if !newest_sample.valid {
            return None;
        }

        let mut previous_sample = newest_sample;
        let mut oldest_sample = newest_sample;

        // Starting with the most recent sample, iterate backwards while the
        // samples represent continuous motion.
        loop {
            let sample = self.samples[index];
            if !sample.valid {
                break;
            }

            let age = millis(newest_sample.time) - millis(sample.time);
            let delta = (millis(sample.time) - millis(previous_sample.time)).abs();
            previous_sample = sample;
            if age > HORIZON_MILLISECONDS || delta > ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS {
                break;
            }

            oldest_sample = sample;
            x[sample_count] = sample.point.x;
            y[sample_count] = sample.point.y;
            w[sample_count] = 1.0;
            time[sample_count] = -age;
            index = if index == 0 { HISTORY_SIZE } else { index } - 1;

            sample_count += 1;
            if sample_count >= HISTORY_SIZE {
                break;
            }
        }

        let offset = newest_sample.point - oldest_sample.point;
        let duration = newest_sample.time.saturating_sub(oldest_sample.time);

        if sample_count >= MIN_SAMPLE_SIZE {
            let x_fit = LeastSquaresSolver::solve(2, &time[..sample_count], &x[..sample_count], &w[..sample_count]);
            if let Some(x_fit) = x_fit {
                let y_fit =
                    LeastSquaresSolver::solve(2, &time[..sample_count], &y[..sample_count], &w[..sample_count]);
                if let Some(y_fit) = y_fit {
                    // convert from pixels/ms to pixels/s
                    return Some(VelocityEstimate {
                        pixels_per_second: Vector::new(x_fit.coefficients[1] * 1000.0, y_fit.coefficients[1] * 1000.0),
                        confidence: x_fit.confidence * y_fit.confidence,
                        duration,
                        offset,
                    });
                }
            }
        }

        // We're unable to make a velocity estimate but we did have at least
        // one valid pointer position.
        Some(VelocityEstimate { pixels_per_second: Vector::default(), confidence: 1.0, duration, offset })
    }

    /// Computes the velocity of the pointer at the time of the last
    /// provided data point.
    ///
    /// This can be expensive. Only call this when you need the velocity.
    /// Returns zero if no velocity could be computed.
    pub fn get_velocity(&self, now: Duration) -> Velocity {
        match self.get_velocity_estimate(now) {
            Some(estimate) => Velocity::new(estimate.pixels_per_second),
            None => Velocity::new(Vector::default()),
        }
    }

    pub fn get_fling_velocity(&self, now: Duration) -> Velocity {
        self.get_velocity(now).clamp_magnitude(MIN_FLING_VELOCITY, MAX_FLING_VELOCITY)
    }
}

/// An nth degree polynomial fit to a dataset.
pub(crate) struct PolynomialFit {
    /// The polynomial coefficients of the fit.
    pub coefficients: Vec<f64>,
    /// An indicator of the quality of the fit.
    ///
    /// The value is in the range 0.0 to 1.0, with 0.0 indicating a poor fit
    /// and 1.0 indicating a perfect fit.
    pub confidence: f64,
}

pub(crate) struct LeastSquaresSolver;

const PRECISION_ERROR_TOLERANCE: f64 = 1e-10;

/// The largest matrices the solver works on: a degree 2 fit over the
/// sample history.
const MAX_ROWS: usize = 3;
const MAX_ELEMENTS: usize = MAX_ROWS * HISTORY_SIZE;

struct Matrix {
    columns: usize,
    elements: [f64; MAX_ELEMENTS],
}

impl Matrix {
    fn new(columns: usize) -> Self {
        Self { columns, elements: [0.0; MAX_ELEMENTS] }
    }

    fn get(&self, row: usize, col: usize) -> f64 {
        self.elements[row * self.columns + col]
    }

    fn set(&mut self, row: usize, col: usize, value: f64) {
        self.elements[row * self.columns + col] = value;
    }

    fn row(&self, row: usize) -> &[f64] {
        &self.elements[row * self.columns..(row + 1) * self.columns]
    }
}

impl LeastSquaresSolver {
    /// Fits a polynomial of the given degree to the data points.
    pub fn solve(degree: usize, x: &[f64], y: &[f64], w: &[f64]) -> Option<PolynomialFit> {
        if degree > x.len() || degree + 1 > MAX_ROWS || x.len() > HISTORY_SIZE {
            // Not enough data to fit a curve.
            return None;
        }

        let mut result = PolynomialFit { coefficients: vec![0.0; degree + 1], confidence: 0.0 };

        // Shorthands for the purpose of notation equivalence to the
        // original code.
        let m = x.len();
        let n = degree + 1;

        // Expand the X vector to a matrix A, pre-multiplied by the weights.
        let mut a = Matrix::new(m);
        for h in 0..m {
            a.set(0, h, w[h]);
            for i in 1..n {
                a.set(i, h, a.get(i - 1, h) * x[h]);
            }
        }

        // Apply the Gram-Schmidt process to A to obtain its QR
        // decomposition.

        // Orthonormal basis, column-major order.
        let mut q = Matrix::new(m);
        // Upper triangular matrix, row-major order.
        let mut r = Matrix::new(n);

        for j in 0..n {
            for h in 0..m {
                q.set(j, h, a.get(j, h));
            }

            for i in 0..j {
                let dot = Self::multiply(q.row(j), q.row(i));
                for h in 0..m {
                    q.set(j, h, q.get(j, h) - dot * q.get(i, h));
                }
            }

            let norm = Self::norm(q.row(j));
            if norm < PRECISION_ERROR_TOLERANCE {
                // Vectors are linearly dependent or zero so no solution.
                return None;
            }

            let inverse_norm = 1.0 / norm;
            for h in 0..m {
                q.set(j, h, q.get(j, h) * inverse_norm);
            }

            for i in 0..n {
                r.set(j, i, if i < j { 0.0 } else { Self::multiply(q.row(j), a.row(i)) });
            }
        }

        // Solve R B = Qt W Y to find B. This is easy because R is upper
        // triangular. We just work from bottom-right to top-left
        // calculating B's coefficients.
        let mut wy = [0.0; HISTORY_SIZE];
        for h in 0..m {
            wy[h] = y[h] * w[h];
        }

        for i in (0..n).rev() {
            result.coefficients[i] = Self::multiply(q.row(i), &wy[..m]);
            for j in ((i + 1)..n).rev() {
                result.coefficients[i] -= r.get(i, j) * result.coefficients[j];
            }
            result.coefficients[i] /= r.get(i, i);
        }

        // Calculate the coefficient of determination (confidence) as:
        //   1 - (sumSquaredError / sumSquaredTotal)
        // ...where sumSquaredError is the residual sum of squares (variance
        // of the error), and sumSquaredTotal is the total sum of squares
        // (variance of the data) where each has been weighted.
        let y_mean = y.iter().sum::<f64>() / m as f64;

        let mut sum_squared_error = 0.0;
        let mut sum_squared_total = 0.0;
        for h in 0..m {
            let mut term = 1.0;
            let mut err = y[h] - result.coefficients[0];
            for i in 1..n {
                term *= x[h];
                err -= term * result.coefficients[i];
            }
            sum_squared_error += w[h] * w[h] * err * err;
            let v = y[h] - y_mean;
            sum_squared_total += w[h] * w[h] * v * v;
        }

        result.confidence = if sum_squared_total <= PRECISION_ERROR_TOLERANCE {
            1.0
        } else {
            1.0 - (sum_squared_error / sum_squared_total)
        };

        Some(result)
    }

    fn multiply(v1: &[f64], v2: &[f64]) -> f64 {
        v1.iter().zip(v2).map(|(a, b)| a * b).sum()
    }

    fn norm(v: &[f64]) -> f64 {
        Self::multiply(v, v).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_a_line() {
        let fit = LeastSquaresSolver::solve(1, &[0.0, 1.0, 2.0], &[1.0, 3.0, 5.0], &[1.0, 1.0, 1.0]).unwrap();
        assert!((fit.coefficients[0] - 1.0).abs() < 1e-9);
        assert!((fit.coefficients[1] - 2.0).abs() < 1e-9);
        assert!((fit.confidence - 1.0).abs() < 1e-9);
        assert!(LeastSquaresSolver::solve(2, &[0.0], &[1.0], &[1.0]).is_none());
    }

    #[test]
    fn estimates_constant_velocity() {
        let mut tracker = VelocityTracker::new();
        for i in 0..5u64 {
            tracker.add_position(Duration::from_millis(i * 10), Vector::new(i as f64 * 5.0, 0.0));
        }

        // 5 pixels per 10ms = 500 pixels per second.
        let velocity = tracker.get_velocity(Duration::from_millis(40));
        assert!((velocity.pixels_per_second.x - 500.0).abs() < 1e-6);
        assert!(velocity.pixels_per_second.y.abs() < 1e-6);

        // The pointer rested for longer than the stop threshold.
        assert_eq!(tracker.get_velocity(Duration::from_millis(100)).pixels_per_second, Vector::default());
        assert_eq!(VelocityTracker::new().get_velocity(Duration::ZERO).pixels_per_second, Vector::default());
    }

    #[test]
    fn fling_velocity_is_clamped() {
        let fast = Velocity::new(Vector::new(20000.0, 0.0)).clamp_magnitude(50.0, 8000.0);
        assert_eq!(fast.pixels_per_second, Vector::new(8000.0, 0.0));
        let slow = Velocity::new(Vector::new(0.0, 10.0)).clamp_magnitude(50.0, 8000.0);
        assert_eq!(slow.pixels_per_second, Vector::new(0.0, 50.0));
        let zero = Velocity::new(Vector::default()).clamp_magnitude(50.0, 8000.0);
        assert_eq!(zero.pixels_per_second, Vector::default());
    }
}
