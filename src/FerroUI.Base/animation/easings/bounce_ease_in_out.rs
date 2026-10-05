use crate::animation::easings::IEasing;
use crate::animation::utils::BounceEaseUtils;

/// Eases a value in and out using a simulated bounce function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BounceEaseInOut;

impl BounceEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for BounceEaseInOut {
    fn ease(&self, progress: f64) -> f64 {
        let p = progress;
        if p < 0.5 {
            0.5 * (1.0 - BounceEaseUtils::bounce(1.0 - (p * 2.0)))
        } else {
            0.5 * BounceEaseUtils::bounce(p * 2.0 - 1.0) + 0.5
        }
    }
}
