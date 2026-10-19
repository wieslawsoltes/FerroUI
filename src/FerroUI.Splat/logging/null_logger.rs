//! `NullLogger`.

use super::{ILogger, LogLevel};
use std::cell::Cell;
use std::error::Error;

/// A logger that writes nothing (`NullLogger`).
#[derive(Default)]
pub struct NullLogger {
    level: Cell<LogLevel>,
}

impl NullLogger {
    /// A logger that writes nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the level (`Level { set; }`).
    pub fn set_level(&self, level: LogLevel) {
        self.level.set(level);
    }
}

impl ILogger for NullLogger {
    fn level(&self) -> LogLevel {
        self.level.get()
    }

    fn write(&self, _message: &str, _log_level: LogLevel) {}

    fn write_exception(&self, _exception: &dyn Error, _message: &str, _log_level: LogLevel) {}

    fn write_with_type(&self, _message: &str, _type_name: &'static str, _log_level: LogLevel) {}

    fn write_exception_with_type(
        &self,
        _exception: &dyn Error,
        _message: &str,
        _type_name: &'static str,
        _log_level: LogLevel,
    ) {
    }
}
