use crate::animation::easings::IEasing;

/// Eases in a value using a quintic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuinticEaseIn;

impl QuinticEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for QuinticEaseIn {
    fn ease(&self, progress: f64) -> f64 {
        let p2 = progress * progress;
        p2 * p2 * progress
    }
}
