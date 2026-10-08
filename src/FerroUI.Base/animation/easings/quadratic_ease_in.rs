use crate::animation::easings::IEasing;

/// Eases in a value using a quadratic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuadraticEaseIn;

impl QuadraticEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for QuadraticEaseIn {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        progress * progress
    }
}
