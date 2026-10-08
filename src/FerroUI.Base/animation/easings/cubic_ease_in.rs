use crate::animation::easings::IEasing;

/// Eases in a value using a cubic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CubicEaseIn;

impl CubicEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for CubicEaseIn {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        progress * progress * progress
    }
}
