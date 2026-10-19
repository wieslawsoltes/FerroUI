//! `TextLogger`.

use super::IMockLogTarget;
use ferroui_splat::{ILogger, LogLevel};
use std::cell::{Cell, RefCell};
use std::error::Error;

/// A logger that records every message, whatever its level (`TextLogger`).
#[derive(Default)]
pub struct TextLogger {
    types: RefCell<Vec<&'static str>>,
    logs: RefCell<Vec<(LogLevel, String)>>,
    level: Cell<LogLevel>,
}

impl TextLogger {
    pub fn new() -> Self {
        Self::default()
    }

    /// A logger at `level` (`new TextLogger { Level = level }`).
    pub fn with_level(level: LogLevel) -> Self {
        let logger = Self::default();
        logger.level.set(level);
        logger
    }

    /// Sets the level (`Level { set; }`).
    pub fn set_level(&self, level: LogLevel) {
        self.level.set(level);
    }

    /// The types the messages were written with.
    pub fn types(&self) -> Vec<&'static str> {
        self.types.borrow().clone()
    }
}

impl IMockLogTarget for TextLogger {
    fn logs(&self) -> Vec<(LogLevel, String)> {
        self.logs.borrow().clone()
    }
}

impl ILogger for TextLogger {
    fn level(&self) -> LogLevel {
        self.level.get()
    }

    fn write(&self, message: &str, log_level: LogLevel) {
        self.logs.borrow_mut().push((log_level, message.to_owned()));
    }

    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel) {
        self.write(&format!("{message} {exception}"), log_level);
    }

    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel) {
        self.logs.borrow_mut().push((log_level, message.to_owned()));
        self.types.borrow_mut().push(type_name);
    }

    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        type_name: &'static str,
        log_level: LogLevel,
    ) {
        self.write_with_type(&format!("{message} {exception}"), type_name, log_level);
    }
}
