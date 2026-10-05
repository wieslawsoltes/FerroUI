use std::time::Duration;

/// Represents a single layout pass timing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LayoutPassTiming {
    /// The number of the layout pass.
    pub pass_counter: i32,
    /// The elapsed time during the layout pass.
    pub elapsed: Duration,
}

impl LayoutPassTiming {
    pub const fn new(pass_counter: i32, elapsed: Duration) -> Self {
        Self { pass_counter, elapsed }
    }
}
