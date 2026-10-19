//! `IStaticFullLogger` and `StaticFullLogger`.
//!
//! This file is written by a script from the list of levels; each level has the same methods.

use super::{IFullLogger, LogLevel};
use std::error::Error;
use std::fmt;
use std::rc::Rc;

/// A logger for static code: each message ends with the name of the member that wrote it,
/// `message (Member)` (`IStaticFullLogger`).
///
/// Upstream fills the name of the member in with `[CallerMemberName]`; Rust has no counterpart, so
/// the caller passes it, and `None` writes `message ()` as a missing name does upstream.
// Deviation (DEVIATIONS.md, Splat): `[CallerMemberName]` is the explicit argument
// `caller_member_name`.
pub trait IStaticFullLogger {
    /// The level from which the logger writes (`Level`).
    fn level(&self) -> LogLevel;

    /// Writes `exception` with `message` at the level `Debug` (`Debug(Exception, string, string?)`).
    fn debug_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` at the level `Debug` (`Debug(string, string?)`).
    fn debug(&self, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` of the type `type_name` at the level `Debug` (`Debug<T>(string, string?)`).
    fn debug_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>);

    /// Writes the message `args` at the level `Debug`
    /// (`Debug<TArgument...>(IFormatProvider, string, TArgument..., string?)`).
    fn debug_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>);

    /// Writes `exception` with `message` at the level `Info` (`Info(Exception, string, string?)`).
    fn info_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` at the level `Info` (`Info(string, string?)`).
    fn info(&self, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` of the type `type_name` at the level `Info` (`Info<T>(string, string?)`).
    fn info_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>);

    /// Writes the message `args` at the level `Info`
    /// (`Info<TArgument...>(IFormatProvider, string, TArgument..., string?)`).
    fn info_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>);

    /// Writes `exception` with `message` at the level `Warn` (`Warn(Exception, string, string?)`).
    fn warn_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` at the level `Warn` (`Warn(string, string?)`).
    fn warn(&self, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` of the type `type_name` at the level `Warn` (`Warn<T>(string, string?)`).
    fn warn_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>);

    /// Writes the message `args` at the level `Warn`
    /// (`Warn<TArgument...>(IFormatProvider, string, TArgument..., string?)`).
    fn warn_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>);

    /// Writes `exception` with `message` at the level `Error` (`Error(Exception, string, string?)`).
    fn error_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` at the level `Error` (`Error(string, string?)`).
    fn error(&self, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` of the type `type_name` at the level `Error` (`Error<T>(string, string?)`).
    fn error_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>);

    /// Writes the message `args` at the level `Error`
    /// (`Error<TArgument...>(IFormatProvider, string, TArgument..., string?)`).
    fn error_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>);

    /// Writes `exception` with `message` at the level `Fatal` (`Fatal(Exception, string, string?)`).
    fn fatal_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` at the level `Fatal` (`Fatal(string, string?)`).
    fn fatal(&self, message: &str, caller_member_name: Option<&str>);

    /// Writes `message` of the type `type_name` at the level `Fatal` (`Fatal<T>(string, string?)`).
    fn fatal_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>);

    /// Writes the message `args` at the level `Fatal`
    /// (`Fatal<TArgument...>(IFormatProvider, string, TArgument..., string?)`).
    fn fatal_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>);

    /// Writes `message` at `log_level` (`Write(string, LogLevel, string?)`).
    fn write(&self, message: &str, log_level: LogLevel, caller_member_name: Option<&str>);

    /// Writes `exception` with `message` at `log_level` (`Write(Exception, string, LogLevel, string?)`).
    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel, caller_member_name: Option<&str>);

    /// Writes `message` of the type `type_name` at `log_level` (`Write(string, Type, LogLevel, string?)`).
    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel, caller_member_name: Option<&str>);

    /// Writes `exception` with `message`, of the type `type_name`, at `log_level`
    /// (`Write(Exception, string, Type, LogLevel, string?)`).
    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        type_name: &'static str,
        log_level: LogLevel,
        caller_member_name: Option<&str>,
    );
}

/// The generic forms of [`IStaticFullLogger`]: the type that writes is a type parameter.
pub trait StaticFullLoggerExt: IStaticFullLogger {
    /// `Debug<T>(string, string?)`.
    fn debug_for<T: ?Sized>(&self, message: &str, caller_member_name: Option<&str>) {
        self.debug_with_type(std::any::type_name::<T>(), message, caller_member_name);
    }

    /// `Info<T>(string, string?)`.
    fn info_for<T: ?Sized>(&self, message: &str, caller_member_name: Option<&str>) {
        self.info_with_type(std::any::type_name::<T>(), message, caller_member_name);
    }

    /// `Warn<T>(string, string?)`.
    fn warn_for<T: ?Sized>(&self, message: &str, caller_member_name: Option<&str>) {
        self.warn_with_type(std::any::type_name::<T>(), message, caller_member_name);
    }

    /// `Error<T>(string, string?)`.
    fn error_for<T: ?Sized>(&self, message: &str, caller_member_name: Option<&str>) {
        self.error_with_type(std::any::type_name::<T>(), message, caller_member_name);
    }

    /// `Fatal<T>(string, string?)`.
    fn fatal_for<T: ?Sized>(&self, message: &str, caller_member_name: Option<&str>) {
        self.fatal_with_type(std::any::type_name::<T>(), message, caller_member_name);
    }
}

impl<L: IStaticFullLogger + ?Sized> StaticFullLoggerExt for L {}

/// The static logger over a full logger (`StaticFullLogger`).
pub struct StaticFullLogger {
    full_logger: Rc<dyn IFullLogger>,
}

impl StaticFullLogger {
    /// A static logger over `full_logger`.
    pub fn new(full_logger: Rc<dyn IFullLogger>) -> Self {
        Self { full_logger }
    }

    /// `GetSuffixedCallerData`: `message (Member)`.
    fn suffixed(message: &str, caller_member_name: Option<&str>) -> String {
        format!("{message} ({})", caller_member_name.unwrap_or(""))
    }
}

impl IStaticFullLogger for StaticFullLogger {
    fn level(&self) -> LogLevel {
        self.full_logger.level()
    }

    fn debug_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.debug_exception(exception, &Self::suffixed(message, caller_member_name));
    }

    fn debug(&self, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.debug(&Self::suffixed(message, caller_member_name));
    }

    fn debug_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.debug_with_type(type_name, &Self::suffixed(message, caller_member_name));
    }

    fn debug_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>) {
        self.full_logger.debug_format(format_args!("{args} ({})", caller_member_name.unwrap_or("")));
    }

    fn info_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.info_exception(exception, &Self::suffixed(message, caller_member_name));
    }

    fn info(&self, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.info(&Self::suffixed(message, caller_member_name));
    }

    fn info_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.info_with_type(type_name, &Self::suffixed(message, caller_member_name));
    }

    fn info_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>) {
        self.full_logger.info_format(format_args!("{args} ({})", caller_member_name.unwrap_or("")));
    }

    fn warn_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.warn_exception(exception, &Self::suffixed(message, caller_member_name));
    }

    fn warn(&self, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.warn(&Self::suffixed(message, caller_member_name));
    }

    fn warn_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.warn_with_type(type_name, &Self::suffixed(message, caller_member_name));
    }

    fn warn_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>) {
        self.full_logger.warn_format(format_args!("{args} ({})", caller_member_name.unwrap_or("")));
    }

    fn error_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.error_exception(exception, &Self::suffixed(message, caller_member_name));
    }

    fn error(&self, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.error(&Self::suffixed(message, caller_member_name));
    }

    fn error_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.error_with_type(type_name, &Self::suffixed(message, caller_member_name));
    }

    fn error_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>) {
        self.full_logger.error_format(format_args!("{args} ({})", caller_member_name.unwrap_or("")));
    }

    fn fatal_exception(&self, exception: &dyn Error, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.fatal_exception(exception, &Self::suffixed(message, caller_member_name));
    }

    fn fatal(&self, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.fatal(&Self::suffixed(message, caller_member_name));
    }

    fn fatal_with_type(&self, type_name: &'static str, message: &str, caller_member_name: Option<&str>) {
        self.full_logger.fatal_with_type(type_name, &Self::suffixed(message, caller_member_name));
    }

    fn fatal_format(&self, args: fmt::Arguments<'_>, caller_member_name: Option<&str>) {
        self.full_logger.fatal_format(format_args!("{args} ({})", caller_member_name.unwrap_or("")));
    }

    fn write(&self, message: &str, log_level: LogLevel, caller_member_name: Option<&str>) {
        self.full_logger.write(&Self::suffixed(message, caller_member_name), log_level);
    }

    fn write_exception(&self, exception: &dyn Error, message: &str, log_level: LogLevel, caller_member_name: Option<&str>) {
        self.full_logger.write_exception(exception, &Self::suffixed(message, caller_member_name), log_level);
    }

    fn write_with_type(&self, message: &str, type_name: &'static str, log_level: LogLevel, caller_member_name: Option<&str>) {
        self.full_logger.write_with_type(&Self::suffixed(message, caller_member_name), type_name, log_level);
    }

    fn write_exception_with_type(
        &self,
        exception: &dyn Error,
        message: &str,
        type_name: &'static str,
        log_level: LogLevel,
        caller_member_name: Option<&str>,
    ) {
        self.full_logger.write_exception_with_type(
            exception,
            &Self::suffixed(message, caller_member_name),
            type_name,
            log_level,
        );
    }
}
