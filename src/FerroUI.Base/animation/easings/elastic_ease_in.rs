use crate::animation::easings::IEasing;
use crate::animation::utils::EasingUtils;

/// Eases in a value using a damped sine function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ElasticEaseIn;

impl ElasticEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for ElasticEaseIn {
    fn ease(&self, progress: f64) -> f64 {
        let p = progress;
        (13.0 * EasingUtils::HALFPI * p).sin() * 2.0_f64.powf(10.0 * (p - 1.0))
    }
}
