use crate::animation::easings::IEasing;

/// Eases in a value using a cubic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CubicEaseIn;

impl CubicEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for CubicEaseIn {
    fn ease(&self, progress: f64) -> f64 {
        progress * progress * progress
    }
}
