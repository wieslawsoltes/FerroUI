use crate::animation::easings::IEasing;

/// Eases out a value using an exponential function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExponentialEaseOut;

impl ExponentialEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for ExponentialEaseOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let p = progress;
        if p == 1.0 {
            p
        } else {
            1.0 - 2.0_f64.powf(-10.0 * p)
        }
    }
}
