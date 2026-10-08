use crate::animation::easings::IEasing;

/// Eases in a value using a quartic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuarticEaseIn;

impl QuarticEaseIn {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for QuarticEaseIn {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let p2 = progress * progress;
        p2 * p2
    }
}
