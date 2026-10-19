//! `WrappingFullLogger`, `WrappingLogLevelLogger` and `WrappingPrefixLogger`.

use super::{
    short_type_name, IAllocationFreeErrorLogger, IAllocationFreeLogger, IFullLogger, ILogger,
    LogLevel,
};
use std::error::Error;
use std::rc::Rc;

/// An [`IFullLogger`] over an [`ILogger`] (`WrappingFullLogger`): each method of the full logger
/// writes to the logger it wraps.
pub struct WrappingFullLogger {
    inner: Rc<dyn ILogger>,
}

impl WrappingFullLogger {
    /// A full logger over `inner`.
    pub fn new(inner: Rc<dyn ILogger>) -> Self {
        Self { inner }
    }
}

impl ILogger for WrappingFullLogger {
    fn level(&self) -> LogLevel {
        self.inner.level()
    }

    fn write(&self, message: &str, log_level: LogLevel) {
        self.inner.write(message, log_level);
    }

    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel) {
        self.inner.write_exception(exception, message, log_level);
    }

    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel) {
        self.inner.write_with_type(message, type_name, log_level);
    }

    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        type_name: &'static str,
        log_level: LogLevel,
    ) {
        self.inner.write_exception_with_type(exception, message, type_name, log_level);
    }
}

impl IAllocationFreeErrorLogger for WrappingFullLogger {}

impl IAllocationFreeLogger for WrappingFullLogger {}

impl IFullLogger for WrappingFullLogger {}

/// A logger that writes each message with its level in front, `Level: message`
/// (`WrappingLogLevelLogger`).
pub struct WrappingLogLevelLogger {
    inner: Rc<dyn ILogger>,
}

impl WrappingLogLevelLogger {
    /// A logger over `inner`.
    pub fn new(inner: Rc<dyn ILogger>) -> Self {
        Self { inner }
    }
}

impl ILogger for WrappingLogLevelLogger {
    fn level(&self) -> LogLevel {
        self.inner.level()
    }

    fn write(&self, message: &str, log_level: LogLevel) {
        self.inner.write(&format!("{log_level}: {message}"), log_level);
    }

    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel) {
        self.inner.write_exception(exception, &format!("{log_level}: {message}"), log_level);
    }

    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel) {
        self.inner.write_with_type(&format!("{log_level}: {message}"), type_name, log_level);
    }

    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        type_name: &'static str,
        log_level: LogLevel,
    ) {
        self.inner
            .write_exception_with_type(exception, &format!("{log_level}: {message}"), type_name, log_level);
    }
}

/// A logger that writes each message with the name of a type in front, `Type: message`
/// (`WrappingPrefixLogger`): the calling type of the logger, or the type a message is written
/// with.
pub struct WrappingPrefixLogger {
    inner: Rc<dyn ILogger>,
    prefix: String,
}

impl WrappingPrefixLogger {
    /// A logger over `inner` for the type `calling_type` (`WrappingPrefixLogger(ILogger, Type)`).
    pub fn new(inner: Rc<dyn ILogger>, calling_type: &'static str) -> Self {
        Self { inner, prefix: format!("{}: ", short_type_name(calling_type)) }
    }
}

impl ILogger for WrappingPrefixLogger {
    fn level(&self) -> LogLevel {
        self.inner.level()
    }

    fn write(&self, message: &str, log_level: LogLevel) {
        self.inner.write(&format!("{}{message}", self.prefix), log_level);
    }

    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel) {
        self.inner.write_exception(exception, &format!("{}{message}", self.prefix), log_level);
    }

    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel) {
        self.inner
            .write_with_type(&format!("{}: {message}", short_type_name(type_name)), type_name, log_level);
    }

    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        type_name: &'static str,
        log_level: LogLevel,
    ) {
        self.inner.write_exception_with_type(
            exception,
            &format!("{}: {message}", short_type_name(type_name)),
            type_name,
            log_level,
        );
    }
}
