use crate::animation::easings::IEasing;

/// Eases out a value using a quadratic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuadraticEaseOut;

impl QuadraticEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for QuadraticEaseOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        -(progress * (progress - 2.0))
    }
}
