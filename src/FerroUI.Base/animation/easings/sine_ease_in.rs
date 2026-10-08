use crate::animation::easings::IEasing;
use crate::animation::utils::EasingUtils;

/// Eases in a value using the quarter-wave of a sine function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SineEaseIn;

impl SineEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for SineEaseIn {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        ((progress - 1.0) * EasingUtils::HALFPI).sin() + 1.0
    }
}
