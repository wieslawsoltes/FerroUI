use crate::animation::easings::IEasing;
use crate::animation::utils::BounceEaseUtils;

/// Eases in a value using a simulated bounce function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BounceEaseIn;

impl BounceEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for BounceEaseIn {
    fn ease(&self, progress: f64) -> f64 {
        1.0 - BounceEaseUtils::bounce(1.0 - progress)
    }
}
