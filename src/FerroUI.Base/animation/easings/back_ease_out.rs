use crate::animation::easings::IEasing;
use std::f64::consts::PI;

/// Eases out a value using a back-off: the value overshoots before settling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackEaseOut;

impl BackEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for BackEaseOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let p = 1.0 - progress;
        1.0 - p * (p * p - (p * PI).sin())
    }
}
