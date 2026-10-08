use crate::animation::easings::IEasing;
use std::f64::consts::PI;

/// Eases in a value using a back-off: the value recedes before moving forward.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackEaseIn;

impl BackEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for BackEaseIn {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        progress * (progress * progress - (progress * PI).sin())
    }
}
