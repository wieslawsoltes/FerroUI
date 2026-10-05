use crate::animation::easings::IEasing;

/// Eases a value in and out using a piece-wise unit circle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CircularEaseInOut;

impl CircularEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for CircularEaseInOut {
    fn ease(&self, progress: f64) -> f64 {
        let p = progress;
        if p < 0.5 {
            0.5 * (1.0 - (1.0 - 4.0 * p * p).sqrt())
        } else {
            let t = 2.0 * p;
            0.5 * (((3.0 - t) * (t - 1.0)).sqrt() + 1.0)
        }
    }
}
