//! `ILogger`.

use super::LogLevel;
use std::error::Error;

/// A logger: writes messages of a level (`ILogger`).
///
/// A message written with a type names the type that wrote it, by its name
/// (`std::any::type_name`).
pub trait ILogger {
    /// The level from which the logger writes (`Level`).
    fn level(&self) -> LogLevel;

    /// Writes `message` (`Write(string, LogLevel)`).
    fn write(&self, message: &str, log_level: LogLevel);

    /// Writes `message` with `exception` (`Write(Exception, string, LogLevel)`).
    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel);

    /// Writes `message` of the type `type_name` (`Write(string, Type, LogLevel)`).
    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel);

    /// Writes `message` with `exception`, of the type `type_name`
    /// (`Write(Exception, string, Type, LogLevel)`).
    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        type_name: &'static str,
        log_level: LogLevel,
    );
}
