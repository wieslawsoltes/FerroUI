use crate::animation::easings::IEasing;
use std::f64::consts::PI;

/// Eases a value in and out using the half-wave of a sine function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SineEaseInOut;

impl SineEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for SineEaseInOut {
    fn ease(&self, progress: f64) -> f64 {
        0.5 * (1.0 - (progress * PI).cos())
    }
}
