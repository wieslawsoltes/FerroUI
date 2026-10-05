use std::cell::Cell;

/// Provides data for the shutdown requested event of an application
/// lifetime.
#[derive(Clone, Debug, Default)]
pub struct ShutdownRequestedEventArgs {
    cancel: Cell<bool>,
    is_os_shutdown: bool,
    will_exit_main_loop: Cell<bool>,
}

impl ShutdownRequestedEventArgs {
    /// Creates the event args.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates the event args for a shutdown that is or is not caused by
    /// the operating system shutting down. For platform backends and the
    /// lifetimes.
    pub fn with_is_os_shutdown(is_os_shutdown: bool) -> Self {
        Self { is_os_shutdown, ..Self::default() }
    }

    /// Whether the event should be canceled.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Sets whether the event should be canceled.
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }

    /// Is the operating system shutting down. For platform backends and the
    /// lifetimes.
    pub fn is_os_shutdown(&self) -> bool {
        self.is_os_shutdown
    }

    /// Indicates that the accepted shutdown will exit the main loop. For
    /// platform backends and the lifetimes.
    pub fn will_exit_main_loop(&self) -> bool {
        self.will_exit_main_loop.get()
    }

    /// Sets whether the accepted shutdown will exit the main loop.
    pub fn set_will_exit_main_loop(&self, value: bool) {
        self.will_exit_main_loop.set(value);
    }
}
