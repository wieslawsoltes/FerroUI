use crate::animation::easings::IEasing;

/// Eases a value in and out using a piece-wise cubic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CubicEaseInOut;

impl CubicEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for CubicEaseInOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let p = progress;

        if progress < 0.5 {
            4.0 * p * p * p
        } else {
            let f = 2.0 * (p - 1.0);
            0.5 * f * f * f + 1.0
        }
    }
}
