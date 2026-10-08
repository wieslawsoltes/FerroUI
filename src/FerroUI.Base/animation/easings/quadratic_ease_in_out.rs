use crate::animation::easings::IEasing;

/// Eases a value in and out using a piece-wise quadratic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuadraticEaseInOut;

impl QuadraticEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for QuadraticEaseInOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let p = progress;

        if progress < 0.5 {
            2.0 * p * p
        } else {
            p * (-2.0 * p + 4.0) - 1.0
        }
    }
}
