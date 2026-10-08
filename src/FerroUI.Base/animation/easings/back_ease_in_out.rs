use crate::animation::easings::IEasing;
use std::f64::consts::PI;

/// Eases a value in and out using a back-off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackEaseInOut;

impl BackEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for BackEaseInOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let p = progress;

        if p < 0.5 {
            let f = 2.0 * p;
            0.5 * f * (f * f - (f * PI).sin())
        } else {
            let f = 1.0 - (2.0 * p - 1.0);
            0.5 * (1.0 - f * (f * f - (f * PI).sin())) + 0.5
        }
    }
}
