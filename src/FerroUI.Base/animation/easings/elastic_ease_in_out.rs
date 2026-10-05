use crate::animation::easings::IEasing;
use crate::animation::utils::EasingUtils;

/// Eases a value in and out using a damped sine function.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ElasticEaseInOut;

impl ElasticEaseInOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for ElasticEaseInOut {
    fn ease(&self, progress: f64) -> f64 {
        let p = progress;

        if p < 0.5 {
            let t = 2.0 * p;
            0.5 * (13.0 * EasingUtils::HALFPI * t).sin() * 2.0_f64.powf(10.0 * (t - 1.0))
        } else {
            0.5 * ((-13.0 * EasingUtils::HALFPI * ((2.0 * p - 1.0) + 1.0)).sin() * 2.0_f64.powf(-10.0 * (2.0 * p - 1.0)) + 2.0)
        }
    }
}
