use crate::animation::easings::IEasing;
use crate::animation::utils::EasingUtils;

/// Eases out a value using a damped sine function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ElasticEaseOut;

impl ElasticEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for ElasticEaseOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let p = progress;
        (-13.0 * EasingUtils::HALFPI * (p + 1.0)).sin() * 2.0_f64.powf(-10.0 * p) + 1.0
    }
}
