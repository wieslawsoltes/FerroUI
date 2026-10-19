//! `ActionLogger`.

use super::{ILogger, LogLevel};
use std::cell::Cell;
use std::error::Error;

type WriteNoType = Box<dyn Fn(&str, LogLevel)>;
type WriteWithType = Box<dyn Fn(&str, &'static str, LogLevel)>;
type WriteNoTypeWithException = Box<dyn Fn(&dyn Error, &str, LogLevel)>;
type WriteWithTypeAndException = Box<dyn Fn(&dyn Error, &str, &'static str, LogLevel)>;

/// A logger that hands each message to a function (`ActionLogger`). It does not filter by level:
/// the functions decide.
pub struct ActionLogger {
    write_no_type: Option<WriteNoType>,
    write_with_type: Option<WriteWithType>,
    write_no_type_with_exception: Option<WriteNoTypeWithException>,
    write_with_type_and_exception: Option<WriteWithTypeAndException>,
    level: Cell<LogLevel>,
}

impl ActionLogger {
    /// A logger over the four functions; a missing one drops its messages
    /// (`ActionLogger(Action<string, LogLevel>, Action<string, Type, LogLevel>,
    /// Action<Exception, string, LogLevel>, Action<Exception, string, Type, LogLevel>)`).
    pub fn new(
        write_no_type: Option<WriteNoType>,
        write_with_type: Option<WriteWithType>,
        write_no_type_with_exception: Option<WriteNoTypeWithException>,
        write_with_type_and_exception: Option<WriteWithTypeAndException>,
    ) -> Self {
        Self {
            write_no_type,
            write_with_type,
            write_no_type_with_exception,
            write_with_type_and_exception,
            level: Cell::new(LogLevel::default()),
        }
    }

    /// Sets the level (`Level { set; }`).
    pub fn set_level(&self, level: LogLevel) {
        self.level.set(level);
    }
}

impl ILogger for ActionLogger {
    fn level(&self) -> LogLevel {
        self.level.get()
    }

    fn write(&self, message: &str, log_level: LogLevel) {
        if let Some(write) = &self.write_no_type {
            write(message, log_level);
        }
    }

    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel) {
        if let Some(write) = &self.write_no_type_with_exception {
            write(exception, message, log_level);
        }
    }

    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel) {
        if let Some(write) = &self.write_with_type {
            write(message, type_name, log_level);
        }
    }

    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        type_name: &'static str,
        log_level: LogLevel,
    ) {
        if let Some(write) = &self.write_with_type_and_exception {
            write(exception, message, type_name, log_level);
        }
    }
}
