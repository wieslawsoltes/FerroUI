use crate::animation::easings::IEasing;

/// Linearly eases a value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinearEasing;

impl LinearEasing {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for LinearEasing {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        progress
    }
}
