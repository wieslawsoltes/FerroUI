use crate::animation::easings::IEasing;

/// Eases out a value using a cubic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CubicEaseOut;

impl CubicEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for CubicEaseOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let f = progress - 1.0;
        f * f * f + 1.0
    }
}
