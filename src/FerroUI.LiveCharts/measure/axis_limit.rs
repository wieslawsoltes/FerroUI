#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AxisLimit {
    pub min: f64,
    pub max: f64,
    pub min_delta: f64,
    pub data_min: f64,
    pub data_max: f64,
    pub user_set_min: Option<f64>,
    pub user_set_max: Option<f64>,
}

impl AxisLimit {
    pub const fn new(min: f64, max: f64, min_delta: f64, data_min: f64, data_max: f64) -> Self {
        Self {
            min,
            max,
            min_delta,
            data_min,
            data_max,
            user_set_min: None,
            user_set_max: None,
        }
    }

    /// Supplies default limits for the sentinel extrema of an empty bounds.
    pub fn validate_limits(min: &mut f64, max: &mut f64, step: f64) {
        let min_defined = *min != f64::MAX && *min != f64::MIN;
        let max_defined = *max != f64::MAX && *max != f64::MIN;
        let scale = if step == 0.0 { 1.0 } else { step };
        match (min_defined, max_defined) {
            (true, true) => {}
            (true, false) => *max = *min + scale * 10.0,
            (false, true) => *min = *max - scale * 10.0,
            (false, false) => {
                *min = 0.0;
                *max = scale * 10.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validate_defined_missing_and_scaled_limits() {
        let limit = AxisLimit::new(0.0, 100.0, 1.0, 5.0, 95.0);
        assert_eq!(
            (
                limit.min,
                limit.max,
                limit.min_delta,
                limit.data_min,
                limit.data_max
            ),
            (0.0, 100.0, 1.0, 5.0, 95.0)
        );
        for (mut min, mut max, step, expected) in [
            (10.0, 100.0, 0.0, (10.0, 100.0)),
            (10.0, f64::MIN, 0.0, (10.0, 20.0)),
            (f64::MAX, 100.0, 0.0, (90.0, 100.0)),
            (f64::MAX, f64::MIN, 0.0, (0.0, 10.0)),
            (f64::MAX, f64::MIN, 5.0, (0.0, 50.0)),
        ] {
            AxisLimit::validate_limits(&mut min, &mut max, step);
            assert_eq!((min, max), expected);
        }
    }
}
