use std::cell::Cell;

/// Contains the arguments for the exit event of a controlled application
/// lifetime.
#[derive(Debug)]
pub struct ControlledApplicationLifetimeExitEventArgs {
    application_exit_code: Cell<i32>,
}

impl ControlledApplicationLifetimeExitEventArgs {
    /// Creates the arguments with the exit code of the application.
    pub fn new(application_exit_code: i32) -> Self {
        Self { application_exit_code: Cell::new(application_exit_code) }
    }

    /// The exit code that should be returned to the operating system when
    /// the application exits.
    pub fn application_exit_code(&self) -> i32 {
        self.application_exit_code.get()
    }

    /// Sets the exit code that should be returned to the operating system
    /// when the application exits.
    pub fn set_application_exit_code(&self, value: i32) {
        self.application_exit_code.set(value)
    }
}
