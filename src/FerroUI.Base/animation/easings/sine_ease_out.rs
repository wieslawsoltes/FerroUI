use crate::animation::easings::IEasing;
use crate::animation::utils::EasingUtils;

/// Eases out a value using the quarter-wave of a sine function with a different phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SineEaseOut;

impl SineEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for SineEaseOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        (progress * EasingUtils::HALFPI).sin()
    }
}
