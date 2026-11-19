use crate::{require, CurvePoint, Result};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CurveMetric {
    #[default]
    Value,
    Velocity,
}

impl CurveMetric {
    /// Velocity uses authored seconds. Each measurement is the signed slope
    /// across neighboring native samples, placed at the interval midpoint.
    pub fn measure(self, points: Vec<CurvePoint>) -> Result<Vec<CurvePoint>> {
        if self == Self::Value {
            return Ok(points);
        }
        let mut velocity = Vec::new();
        let mut gap = false;
        for pair in points.windows(2) {
            let from = pair[0];
            let to = pair[1];
            if to.break_before {
                gap = true;
                continue;
            }
            let seconds = to.time - from.time;
            // The uniform grid and per-segment samples can land a few ULPs apart.
            if !seconds.is_finite() || seconds <= to.time.abs() * (8.0 * f64::EPSILON) {
                continue;
            }
            let change = to.value - from.value;
            let value = if change.is_finite() {
                change / seconds
            } else {
                to.value / seconds - from.value / seconds
            };
            if value.is_finite() {
                velocity.push(CurvePoint {
                    break_before: std::mem::take(&mut gap) && !velocity.is_empty(),
                    ..CurvePoint::new(from.time + seconds * 0.5, value)
                });
            }
        }
        require(
            !velocity.is_empty(),
            "Choose a time range with measurable velocity",
        )?;
        Ok(velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_preserves_direction_and_uses_authored_seconds() {
        let points = vec![
            CurvePoint::new(0.0, 2.0),
            CurvePoint::new(0.5, 6.0),
            CurvePoint::new(1.5, 4.0),
            CurvePoint::new(2.0, 4.0),
        ];
        assert_eq!(CurveMetric::Value.measure(points.clone()).unwrap(), points);
        assert_eq!(
            CurveMetric::Velocity.measure(points).unwrap(),
            vec![
                CurvePoint::new(0.25, 8.0),
                CurvePoint::new(1.0, -2.0),
                CurvePoint::new(1.75, 0.0)
            ]
        );
    }

    #[test]
    fn velocity_skips_rounded_duplicate_times_and_handles_wide_values() {
        let points = vec![
            CurvePoint::new(1.0, -f64::MAX),
            CurvePoint::new(1.0 + f64::EPSILON, -f64::MAX),
            CurvePoint::new(5.0, f64::MAX),
        ];
        let velocity = CurveMetric::Velocity.measure(points.clone()).unwrap();
        assert_eq!(velocity.len(), 1);
        assert_eq!(velocity[0].value, f64::MAX / 2.0);
        assert!(CurveMetric::Velocity.measure(points[..2].to_vec()).is_err());
    }
}
