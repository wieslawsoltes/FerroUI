use crate::animation::easings::IEasing;
use crate::animation::utils::BounceEaseUtils;

/// Eases out a value using a simulated bounce function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BounceEaseOut;

impl BounceEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for BounceEaseOut {
    fn ease(&self, progress: f64) -> f64 {
        BounceEaseUtils::bounce(progress)
    }
}
