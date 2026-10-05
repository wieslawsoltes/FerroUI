use std::time::Duration;

/// Options for the dispatcher, registered with the service locator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DispatcherOptions {
    /// Gets or sets a timeout after which the dispatcher will start
    /// prioritizing input events over rendering. The default value is one
    /// second.
    ///
    /// If no input events are processed within this time, the dispatcher
    /// will start prioritizing input events over rendering to prevent the
    /// application from becoming unresponsive. This may need to be lowered on
    /// resource-constrained platforms where input events are processed on
    /// the same thread as rendering.
    pub input_starvation_timeout: Duration,
}

impl Default for DispatcherOptions {
    fn default() -> Self {
        Self { input_starvation_timeout: Duration::from_secs(1) }
    }
}
