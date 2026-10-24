use crate::easing_functions::{EasingFunction, EasingFunctions};
use std::rc::Rc;

/// The number of repetitions after the initial cycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    None,
    Count(u32),
    Forever,
}

#[derive(Clone)]
pub struct Animation {
    /// Milliseconds, on the caller's monotonic clock.
    pub duration: f64,
    /// `None` disables interpolation and snaps to the target.
    pub easing_function: Option<EasingFunction>,
    pub repeat_times: Repeat,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            duration: 0.0,
            easing_function: Some(Rc::new(EasingFunctions::lineal)),
            repeat_times: Repeat::None,
        }
    }
}

impl Animation {
    pub fn new(duration: f64, easing: impl Fn(f64) -> f64 + 'static) -> Self {
        Self {
            duration,
            easing_function: Some(Rc::new(easing)),
            repeat_times: Repeat::None,
        }
    }
    pub fn with_repeat(mut self, repeat: Repeat) -> Self {
        self.repeat_times = repeat;
        self
    }
}

impl std::fmt::Debug for Animation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Animation")
            .field("duration", &self.duration)
            .field("has_easing", &self.easing_function.is_some())
            .field("repeat_times", &self.repeat_times)
            .finish()
    }
}
