use super::Layoutable;

/// The effective minimum and maximum size of a layoutable, combining its
/// explicit size with its min/max constraints.
#[derive(Clone, Copy, Debug)]
pub struct MinMax {
    pub min_width: f64,
    pub max_width: f64,
    pub min_height: f64,
    pub max_height: f64,
}

impl MinMax {
    pub fn new(e: &Layoutable) -> Self {
        let (min_width, max_width) = Self::calc_min_max(e.width(), e.min_width(), e.max_width());
        let (min_height, max_height) = Self::calc_min_max(e.height(), e.min_height(), e.max_height());
        Self { min_width, max_width, min_height, max_height }
    }

    fn calc_min_max(value: f64, min: f64, max: f64) -> (f64, f64) {
        let (v0, v1) = if value.is_nan() { (0.0, f64::INFINITY) } else { (value, value) };
        let max = Self::clamp_unchecked(v1, min, max);
        let min = Self::clamp_unchecked(v0, min, max);
        (min, max)
    }

    // Not `f64::clamp`: it's possible for min to be greater than max here.
    #[inline]
    fn clamp_unchecked(mut value: f64, min: f64, max: f64) -> f64 {
        if value > max {
            value = max;
        }
        if value < min {
            value = min;
        }
        value
    }
}
