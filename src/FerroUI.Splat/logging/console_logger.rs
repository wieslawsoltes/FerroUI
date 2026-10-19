//! `ConsoleLogger`.

use super::{ILogger, LogLevel};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::io::Write;

/// A logger that writes a line per message to the standard output or to a writer
/// (`ConsoleLogger`), the messages of its level and above.
///
/// A message with an exception is written through [`exception_message_format`]: `{0}` is the
/// message and `{1}` the exception.
///
/// [`exception_message_format`]: Self::exception_message_format
// Deviation (DEVIATIONS.md, Splat): the `TextWriter` is a `std::io::Write`; a failure to write is
// ignored, as a logger has no one to report it to.
pub struct ConsoleLogger {
    writer: RefCell<Box<dyn Write>>,
    exception_message_format: RefCell<String>,
    level: Cell<LogLevel>,
}

impl Default for ConsoleLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleLogger {
    /// A logger that writes to the standard output (`ConsoleLogger()`).
    pub fn new() -> Self {
        Self::with_writer(Box::new(std::io::stdout()))
    }

    /// A logger that writes to `writer` (`ConsoleLogger(TextWriter)`).
    pub fn with_writer(writer: Box<dyn Write>) -> Self {
        Self {
            writer: RefCell::new(writer),
            exception_message_format: RefCell::new("{0} - {1}".to_owned()),
            level: Cell::new(LogLevel::default()),
        }
    }

    /// The format of a message with an exception (`ExceptionMessageFormat`), `{0} - {1}` at
    /// first.
    pub fn exception_message_format(&self) -> String {
        self.exception_message_format.borrow().clone()
    }

    /// Sets the format of a message with an exception.
    pub fn set_exception_message_format(&self, format: impl Into<String>) {
        *self.exception_message_format.borrow_mut() = format.into();
    }

    /// Sets the level (`Level { set; }`).
    pub fn set_level(&self, level: LogLevel) {
        self.level.set(level);
    }

    fn write_line(&self, line: &str) {
        let mut writer = self.writer.borrow_mut();
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }

    fn format_exception_message(&self, message: &str, exception: &dyn Error) -> String {
        format_two(&self.exception_message_format.borrow(), message, &exception.to_string())
    }
}

/// `string.Format(format, first, second)` for a format whose items are `{0}` and `{1}`; `{{` and
/// `}}` are braces.
fn format_two(format: &str, first: &str, second: &str) -> String {
    let mut result = String::with_capacity(format.len() + first.len() + second.len());
    let mut rest = format;
    while let Some(index) = rest.find(['{', '}']) {
        result.push_str(&rest[..index]);
        let tail = &rest[index..];
        if tail.starts_with("{{") {
            result.push('{');
            rest = &tail[2..];
        } else if tail.starts_with("}}") {
            result.push('}');
            rest = &tail[2..];
        } else if tail.starts_with("{0}") {
            result.push_str(first);
            rest = &tail[3..];
        } else if tail.starts_with("{1}") {
            result.push_str(second);
            rest = &tail[3..];
        } else {
            result.push_str(&tail[..1]);
            rest = &tail[1..];
        }
    }
    result.push_str(rest);
    result
}

impl ILogger for ConsoleLogger {
    fn level(&self) -> LogLevel {
        self.level.get()
    }

    fn write(&self, message: &str, log_level: LogLevel) {
        if log_level < self.level.get() {
            return;
        }
        self.write_line(message);
    }

    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel) {
        if log_level < self.level.get() {
            return;
        }
        self.write_line(&self.format_exception_message(message, exception));
    }

    fn write_with_type(&self, message: &str, _type_name: &'static str, log_level: LogLevel) {
        if log_level < self.level.get() {
            return;
        }
        self.write_line(message);
    }

    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        _type_name: &'static str,
        log_level: LogLevel,
    ) {
        if log_level < self.level.get() {
            return;
        }
        self.write_line(&self.format_exception_message(message, exception));
    }
}
