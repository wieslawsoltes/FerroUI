use std::time::Duration;

/// The position to draw and whether an animation crossed its loop boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizeResult {
    pub effective_elapsed: Duration,
    pub has_looped: bool,
}

impl NormalizeResult {
    pub const fn new(effective_elapsed: Duration, has_looped: bool) -> Self {
        Self {
            effective_elapsed,
            has_looped,
        }
    }

    pub const fn create(effective_elapsed: Duration, has_looped: bool) -> Self {
        Self::new(effective_elapsed, has_looped)
    }
}
