use crate::animation::easings::IEasing;

/// Eases in a value using the shifted fourth quadrant of the unit circle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CircularEaseIn;

impl CircularEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for CircularEaseIn {
    fn ease(&self, progress: f64) -> f64 {
        1.0 - (1.0 - progress * progress).sqrt()
    }
}
