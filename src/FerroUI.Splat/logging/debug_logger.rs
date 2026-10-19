//! `DebugLogger`.

use super::{short_type_name, ILogger, LogLevel};
use std::cell::Cell;
use std::error::Error;

/// A logger that writes to the debug output (`DebugLogger`), the messages of its level and above.
///
/// A message with a type is written with the name of the type as its category, as
/// `Debug.WriteLine(message, category)` writes it: `Category: message`.
// Deviation (DEVIATIONS.md, Splat): the debug output of .NET (`System.Diagnostics.Debug`) is the
// standard error here, in every build.
#[derive(Default)]
pub struct DebugLogger {
    level: Cell<LogLevel>,
}

impl DebugLogger {
    /// A logger at the level `Debug`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the level (`Level { set; }`).
    pub fn set_level(&self, level: LogLevel) {
        self.level.set(level);
    }

    fn write_line(message: &str, category: Option<&str>) {
        match category {
            Some(category) => eprintln!("{category}: {message}"),
            None => eprintln!("{message}"),
        }
    }
}

impl ILogger for DebugLogger {
    fn level(&self) -> LogLevel {
        self.level.get()
    }

    fn write(&self, message: &str, log_level: LogLevel) {
        if log_level < self.level.get() {
            return;
        }
        Self::write_line(message, None);
    }

    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel) {
        if log_level < self.level.get() {
            return;
        }
        Self::write_line(&format!("{message} - {exception}"), None);
    }

    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel) {
        if log_level < self.level.get() {
            return;
        }
        Self::write_line(message, Some(short_type_name(type_name)));
    }

    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        type_name: &'static str,
        log_level: LogLevel,
    ) {
        if log_level < self.level.get() {
            return;
        }
        Self::write_line(&format!("{message} - {exception}"), Some(short_type_name(type_name)));
    }
}
