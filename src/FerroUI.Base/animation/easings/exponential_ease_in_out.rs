use crate::animation::easings::IEasing;

/// Eases a value in and out using a piece-wise exponential function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExponentialEaseInOut;

impl ExponentialEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for ExponentialEaseInOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let p = progress;

        if p < 0.5 {
            0.5 * 2.0_f64.powf(20.0 * p - 10.0)
        } else {
            -0.5 * 2.0_f64.powf(-20.0 * p + 10.0) + 1.0
        }
    }
}
