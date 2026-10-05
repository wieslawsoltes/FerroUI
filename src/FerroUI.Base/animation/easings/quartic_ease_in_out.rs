use crate::animation::easings::IEasing;

/// Eases a value in and out using a piece-wise quartic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuarticEaseInOut;

impl QuarticEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for QuarticEaseInOut {
    fn ease(&self, progress: f64) -> f64 {
        let p = progress;

        if p < 0.5 {
            let p2 = p * p;
            8.0 * p2 * p2
        } else {
            let f = p - 1.0;
            let f2 = f * f;
            -8.0 * f2 * f2 + 1.0
        }
    }
}
