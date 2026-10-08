use crate::animation::easings::IEasing;

/// Eases out a value using a quintic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuinticEaseOut;

impl QuinticEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for QuinticEaseOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let f = progress - 1.0;
        let f2 = f * f;
        f2 * f2 * f + 1.0
    }
}
