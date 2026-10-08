use crate::animation::easings::IEasing;

/// Eases out a value using a quartic equation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuarticEaseOut;

impl QuarticEaseOut {
    pub fn new() -> Self {
        Self
    }
}

impl IEasing for QuarticEaseOut {
    fn to_shared(&self) -> std::sync::Arc<crate::animation::easings::SharedEasing> {
        std::sync::Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        let f = progress - 1.0;
        let f2 = f * f;
        -f2 * f2 + 1.0
    }
}
