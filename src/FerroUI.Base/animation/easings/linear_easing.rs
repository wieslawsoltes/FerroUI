use crate::animation::easings::IEasing;

/// Linearly eases a value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinearEasing;

impl LinearEasing {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for LinearEasing {
    fn ease(&self, progress: f64) -> f64 {
        progress
    }
}
