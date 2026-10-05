use crate::animation::easings::IEasing;

/// Eases a value in and out using a piece-wise quintic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuinticEaseInOut;

impl QuinticEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for QuinticEaseInOut {
    fn ease(&self, progress: f64) -> f64 {
        let p = progress;
        if p < 0.5 {
            let p2 = p * p;
            16.0 * p2 * p2 * p
        } else {
            let f = 2.0 * p - 2.0;
            let f2 = f * f;
            0.5 * f2 * f2 * f + 1.0
        }
    }
}
