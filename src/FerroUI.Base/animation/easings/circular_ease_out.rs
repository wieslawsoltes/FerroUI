use crate::animation::easings::IEasing;

/// Eases out a value using the shifted second quadrant of the unit circle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CircularEaseOut;

impl CircularEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for CircularEaseOut {
    fn ease(&self, progress: f64) -> f64 {
        let p = progress;
        ((2.0 - p) * p).sqrt()
    }
}
