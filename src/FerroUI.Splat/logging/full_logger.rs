//! `IAllocationFreeErrorLogger`, `IAllocationFreeLogger` and `IFullLogger`, with the behaviour of
//! `AllocationFreeLoggerBase` and `WrappingFullLogger` as the provided methods of the traits.
//!
//! This file is written by a script from the list of levels; each level has the same methods.

use super::{ILogger, LogLevel};
use std::error::Error;
use std::fmt;

// Deviation (DEVIATIONS.md, Splat): upstream implements these interfaces in
// `AllocationFreeLoggerBase` and `WrappingFullLogger`; here their bodies are the provided methods
// of the traits, written in terms of `ILogger`, so that `WrappingFullLogger` only forwards
// `ILogger` to the logger it wraps.

/// The allocation-free logging of an exception (`IAllocationFreeErrorLogger`): the exception and a
/// message formatted from arguments, formatted only when the level is enabled.
///
/// The ten generated overloads per level of upstream (`Debug<TArgument>(Exception, string,
/// TArgument)`, ...) are one method per level that takes `format_args!`.
pub trait IAllocationFreeErrorLogger: ILogger {
    /// Writes `exception` with the message `args` at the level `Debug`, when the level is enabled
    /// (`Debug<TArgument...>(Exception, string, TArgument...)`).
    fn debug_exception_args(&self, exception: &dyn Error, args: fmt::Arguments<'_>) {
        if self.level() <= LogLevel::Debug {
            self.write_exception(exception, &args.to_string(), LogLevel::Debug);
        }
    }

    /// Writes `exception` with the message `args` at the level `Info`, when the level is enabled
    /// (`Info<TArgument...>(Exception, string, TArgument...)`).
    fn info_exception_args(&self, exception: &dyn Error, args: fmt::Arguments<'_>) {
        if self.level() <= LogLevel::Info {
            self.write_exception(exception, &args.to_string(), LogLevel::Info);
        }
    }

    /// Writes `exception` with the message `args` at the level `Warn`, when the level is enabled
    /// (`Warn<TArgument...>(Exception, string, TArgument...)`).
    fn warn_exception_args(&self, exception: &dyn Error, args: fmt::Arguments<'_>) {
        if self.level() <= LogLevel::Warn {
            self.write_exception(exception, &args.to_string(), LogLevel::Warn);
        }
    }

    /// Writes `exception` with the message `args` at the level `Error`, when the level is enabled
    /// (`Error<TArgument...>(Exception, string, TArgument...)`).
    fn error_exception_args(&self, exception: &dyn Error, args: fmt::Arguments<'_>) {
        if self.level() <= LogLevel::Error {
            self.write_exception(exception, &args.to_string(), LogLevel::Error);
        }
    }

    /// Writes `exception` with the message `args` at the level `Fatal`, when the level is enabled
    /// (`Fatal<TArgument...>(Exception, string, TArgument...)`).
    fn fatal_exception_args(&self, exception: &dyn Error, args: fmt::Arguments<'_>) {
        if self.level() <= LogLevel::Fatal {
            self.write_exception(exception, &args.to_string(), LogLevel::Fatal);
        }
    }
}

/// The allocation-free logging of messages (`IAllocationFreeLogger`): whether each level is enabled,
/// and a message formatted from arguments, formatted only when the level is enabled.
///
/// The ten generated overloads per level of upstream (`Debug<TArgument>(string, TArgument)`, ...)
/// are one method per level that takes `format_args!`.
pub trait IAllocationFreeLogger: IAllocationFreeErrorLogger {
    /// Whether the level `Debug` is enabled (`IsDebugEnabled`).
    fn is_debug_enabled(&self) -> bool {
        self.level() <= LogLevel::Debug
    }

    /// Whether the level `Info` is enabled (`IsInfoEnabled`).
    fn is_info_enabled(&self) -> bool {
        self.level() <= LogLevel::Info
    }

    /// Whether the level `Warn` is enabled (`IsWarnEnabled`).
    fn is_warn_enabled(&self) -> bool {
        self.level() <= LogLevel::Warn
    }

    /// Whether the level `Error` is enabled (`IsErrorEnabled`).
    fn is_error_enabled(&self) -> bool {
        self.level() <= LogLevel::Error
    }

    /// Whether the level `Fatal` is enabled (`IsFatalEnabled`).
    fn is_fatal_enabled(&self) -> bool {
        self.level() <= LogLevel::Fatal
    }

    /// Writes the message `args` at the level `Debug`, when the level is enabled
    /// (`Debug<TArgument...>(string, TArgument...)`).
    fn debug_args(&self, args: fmt::Arguments<'_>) {
        if self.is_debug_enabled() {
            self.write(&args.to_string(), LogLevel::Debug);
        }
    }

    /// Writes the message `args` at the level `Info`, when the level is enabled
    /// (`Info<TArgument...>(string, TArgument...)`).
    fn info_args(&self, args: fmt::Arguments<'_>) {
        if self.is_info_enabled() {
            self.write(&args.to_string(), LogLevel::Info);
        }
    }

    /// Writes the message `args` at the level `Warn`, when the level is enabled
    /// (`Warn<TArgument...>(string, TArgument...)`).
    fn warn_args(&self, args: fmt::Arguments<'_>) {
        if self.is_warn_enabled() {
            self.write(&args.to_string(), LogLevel::Warn);
        }
    }

    /// Writes the message `args` at the level `Error`, when the level is enabled
    /// (`Error<TArgument...>(string, TArgument...)`).
    fn error_args(&self, args: fmt::Arguments<'_>) {
        if self.is_error_enabled() {
            self.write(&args.to_string(), LogLevel::Error);
        }
    }

    /// Writes the message `args` at the level `Fatal`, when the level is enabled
    /// (`Fatal<TArgument...>(string, TArgument...)`).
    fn fatal_args(&self, args: fmt::Arguments<'_>) {
        if self.is_fatal_enabled() {
            self.write(&args.to_string(), LogLevel::Fatal);
        }
    }
}

/// A logger with a method per level and kind of message (`IFullLogger`).
///
/// As upstream's `WrappingFullLogger`, the methods that take a value or a function check the level
/// first, so that nothing is formatted or computed for a level that is disabled; the methods that
/// take a message hand it to the logger, which filters by level.
///
/// The overloads with an `IFormatProvider` are the `_format` methods: Rust's formatting does not
/// depend on a culture.
// Deviation (DEVIATIONS.md, Splat): the overloads with an `IFormatProvider` have no counterpart of
// their own; a `null` message does not exist (`&str`).
pub trait IFullLogger: IAllocationFreeLogger {
    /// Writes the text of `value` at the level `Debug`, when the level is enabled (`Debug<T>(T value)`).
    fn debug_value(&self, value: &dyn fmt::Display) {
        if self.is_debug_enabled() {
            self.write(&value.to_string(), LogLevel::Debug);
        }
    }

    /// Writes `exception` with `message` at the level `Debug` (`Debug(Exception, string?)`).
    fn debug_exception(&self, exception: &dyn Error, message: &str) {
        self.write_exception(exception, message, LogLevel::Debug);
    }

    /// Writes `message` at the level `Debug` (`Debug(string?)`).
    fn debug(&self, message: &str) {
        self.write(message, LogLevel::Debug);
    }

    /// Writes `message` of the type `type_name` at the level `Debug` (`Debug<T>(string?)`).
    fn debug_with_type(&self, type_name: &'static str, message: &str) {
        self.write_with_type(message, type_name, LogLevel::Debug);
    }

    /// Writes the message `args` at the level `Debug` (`Debug(string, params object[])`,
    /// `Debug(IFormatProvider, string, params object[])`, `Debug<T>(IFormatProvider, T)` and
    /// `Debug<TArgument...>(IFormatProvider, string, TArgument...)`).
    fn debug_format(&self, args: fmt::Arguments<'_>) {
        self.write(&args.to_string(), LogLevel::Debug);
    }

    /// Writes the message `args` of the type `type_name` at the level `Debug`
    /// (`Debug<T>(string, params object[])`).
    fn debug_format_with_type(&self, type_name: &'static str, args: fmt::Arguments<'_>) {
        self.write_with_type(&args.to_string(), type_name, LogLevel::Debug);
    }

    /// Writes the message `function` returns at the level `Debug`; the function is called only when
    /// the level is enabled (`Debug(Func<string>)`).
    fn debug_fn(&self, function: &dyn Fn() -> String) {
        if self.is_debug_enabled() {
            self.write(&function(), LogLevel::Debug);
        }
    }

    /// Writes the message `function` returns, of the type `type_name`, at the level `Debug`; the
    /// function is called only when the level is enabled (`Debug<T>(Func<string>)`).
    fn debug_fn_with_type(&self, type_name: &'static str, function: &dyn Fn() -> String) {
        if self.is_debug_enabled() {
            self.write_with_type(&function(), type_name, LogLevel::Debug);
        }
    }

    /// Writes `exception` with the message `function` returns at the level `Debug`; the function is
    /// called only when the level is enabled (`Debug(Exception, Func<string>)`).
    fn debug_exception_fn(&self, exception: &dyn Error, function: &dyn Fn() -> String) {
        if self.is_debug_enabled() {
            self.write_exception(exception, &function(), LogLevel::Debug);
        }
    }

    /// Writes `exception` with `message: exception` at the level `Debug`
    /// (`DebugException(string?, Exception)`).
    #[deprecated(note = "Use debug_exception(exception, message)")]
    fn debug_exception_message(&self, message: &str, exception: &dyn Error) {
        self.write_exception(exception, &format!("{message}: {exception}"), LogLevel::Debug);
    }

    /// Writes `exception` with `message: exception`, the message returned by `function`, at the
    /// level `Debug`; the function is called only when the level is enabled
    /// (`DebugException(Func<string>, Exception)`).
    #[deprecated(note = "Use debug_exception_fn(exception, function)")]
    fn debug_exception_message_fn(&self, function: &dyn Fn() -> String, exception: &dyn Error) {
        if self.is_debug_enabled() {
            self.write_exception(exception, &format!("{}: {exception}", function()), LogLevel::Debug);
        }
    }

    /// Writes the text of `value` at the level `Info`, when the level is enabled (`Info<T>(T value)`).
    fn info_value(&self, value: &dyn fmt::Display) {
        if self.is_info_enabled() {
            self.write(&value.to_string(), LogLevel::Info);
        }
    }

    /// Writes `exception` with `message` at the level `Info` (`Info(Exception, string?)`).
    fn info_exception(&self, exception: &dyn Error, message: &str) {
        self.write_exception(exception, message, LogLevel::Info);
    }

    /// Writes `message` at the level `Info` (`Info(string?)`).
    fn info(&self, message: &str) {
        self.write(message, LogLevel::Info);
    }

    /// Writes `message` of the type `type_name` at the level `Info` (`Info<T>(string?)`).
    fn info_with_type(&self, type_name: &'static str, message: &str) {
        self.write_with_type(message, type_name, LogLevel::Info);
    }

    /// Writes the message `args` at the level `Info` (`Info(string, params object[])`,
    /// `Info(IFormatProvider, string, params object[])`, `Info<T>(IFormatProvider, T)` and
    /// `Info<TArgument...>(IFormatProvider, string, TArgument...)`).
    fn info_format(&self, args: fmt::Arguments<'_>) {
        self.write(&args.to_string(), LogLevel::Info);
    }

    /// Writes the message `args` of the type `type_name` at the level `Info`
    /// (`Info<T>(string, params object[])`).
    fn info_format_with_type(&self, type_name: &'static str, args: fmt::Arguments<'_>) {
        self.write_with_type(&args.to_string(), type_name, LogLevel::Info);
    }

    /// Writes the message `function` returns at the level `Info`; the function is called only when
    /// the level is enabled (`Info(Func<string>)`).
    fn info_fn(&self, function: &dyn Fn() -> String) {
        if self.is_info_enabled() {
            self.write(&function(), LogLevel::Info);
        }
    }

    /// Writes the message `function` returns, of the type `type_name`, at the level `Info`; the
    /// function is called only when the level is enabled (`Info<T>(Func<string>)`).
    fn info_fn_with_type(&self, type_name: &'static str, function: &dyn Fn() -> String) {
        if self.is_info_enabled() {
            self.write_with_type(&function(), type_name, LogLevel::Info);
        }
    }

    /// Writes `exception` with the message `function` returns at the level `Info`; the function is
    /// called only when the level is enabled (`Info(Exception, Func<string>)`).
    fn info_exception_fn(&self, exception: &dyn Error, function: &dyn Fn() -> String) {
        if self.is_info_enabled() {
            self.write_exception(exception, &function(), LogLevel::Info);
        }
    }

    /// Writes `exception` with `message: exception` at the level `Info`
    /// (`InfoException(string?, Exception)`).
    #[deprecated(note = "Use info_exception(exception, message)")]
    fn info_exception_message(&self, message: &str, exception: &dyn Error) {
        self.write_exception(exception, &format!("{message}: {exception}"), LogLevel::Info);
    }

    /// Writes `exception` with `message: exception`, the message returned by `function`, at the
    /// level `Info`; the function is called only when the level is enabled
    /// (`InfoException(Func<string>, Exception)`).
    #[deprecated(note = "Use info_exception_fn(exception, function)")]
    fn info_exception_message_fn(&self, function: &dyn Fn() -> String, exception: &dyn Error) {
        if self.is_info_enabled() {
            self.write_exception(exception, &format!("{}: {exception}", function()), LogLevel::Info);
        }
    }

    /// Writes the text of `value` at the level `Warn`, when the level is enabled (`Warn<T>(T value)`).
    fn warn_value(&self, value: &dyn fmt::Display) {
        if self.is_warn_enabled() {
            self.write(&value.to_string(), LogLevel::Warn);
        }
    }

    /// Writes `exception` with `message` at the level `Warn` (`Warn(Exception, string?)`).
    fn warn_exception(&self, exception: &dyn Error, message: &str) {
        self.write_exception(exception, message, LogLevel::Warn);
    }

    /// Writes `message` at the level `Warn` (`Warn(string?)`).
    fn warn(&self, message: &str) {
        self.write(message, LogLevel::Warn);
    }

    /// Writes `message` of the type `type_name` at the level `Warn` (`Warn<T>(string?)`).
    fn warn_with_type(&self, type_name: &'static str, message: &str) {
        self.write_with_type(message, type_name, LogLevel::Warn);
    }

    /// Writes the message `args` at the level `Warn` (`Warn(string, params object[])`,
    /// `Warn(IFormatProvider, string, params object[])`, `Warn<T>(IFormatProvider, T)` and
    /// `Warn<TArgument...>(IFormatProvider, string, TArgument...)`).
    fn warn_format(&self, args: fmt::Arguments<'_>) {
        self.write(&args.to_string(), LogLevel::Warn);
    }

    /// Writes the message `args` of the type `type_name` at the level `Warn`
    /// (`Warn<T>(string, params object[])`).
    fn warn_format_with_type(&self, type_name: &'static str, args: fmt::Arguments<'_>) {
        self.write_with_type(&args.to_string(), type_name, LogLevel::Warn);
    }

    /// Writes the message `function` returns at the level `Warn`; the function is called only when
    /// the level is enabled (`Warn(Func<string>)`).
    fn warn_fn(&self, function: &dyn Fn() -> String) {
        if self.is_warn_enabled() {
            self.write(&function(), LogLevel::Warn);
        }
    }

    /// Writes the message `function` returns, of the type `type_name`, at the level `Warn`; the
    /// function is called only when the level is enabled (`Warn<T>(Func<string>)`).
    fn warn_fn_with_type(&self, type_name: &'static str, function: &dyn Fn() -> String) {
        if self.is_warn_enabled() {
            self.write_with_type(&function(), type_name, LogLevel::Warn);
        }
    }

    /// Writes `exception` with the message `function` returns at the level `Warn`; the function is
    /// called only when the level is enabled (`Warn(Exception, Func<string>)`).
    fn warn_exception_fn(&self, exception: &dyn Error, function: &dyn Fn() -> String) {
        if self.is_warn_enabled() {
            self.write_exception(exception, &function(), LogLevel::Warn);
        }
    }

    /// Writes `exception` with `message: exception` at the level `Warn`
    /// (`WarnException(string?, Exception)`).
    #[deprecated(note = "Use warn_exception(exception, message)")]
    fn warn_exception_message(&self, message: &str, exception: &dyn Error) {
        self.write_exception(exception, &format!("{message}: {exception}"), LogLevel::Warn);
    }

    /// Writes `exception` with `message: exception`, the message returned by `function`, at the
    /// level `Warn`; the function is called only when the level is enabled
    /// (`WarnException(Func<string>, Exception)`).
    #[deprecated(note = "Use warn_exception_fn(exception, function)")]
    fn warn_exception_message_fn(&self, function: &dyn Fn() -> String, exception: &dyn Error) {
        if self.is_warn_enabled() {
            self.write_exception(exception, &format!("{}: {exception}", function()), LogLevel::Warn);
        }
    }

    /// Writes the text of `value` at the level `Error`, when the level is enabled (`Error<T>(T value)`).
    fn error_value(&self, value: &dyn fmt::Display) {
        if self.is_error_enabled() {
            self.write(&value.to_string(), LogLevel::Error);
        }
    }

    /// Writes `exception` with `message` at the level `Error` (`Error(Exception, string?)`).
    fn error_exception(&self, exception: &dyn Error, message: &str) {
        self.write_exception(exception, message, LogLevel::Error);
    }

    /// Writes `message` at the level `Error` (`Error(string?)`).
    fn error(&self, message: &str) {
        self.write(message, LogLevel::Error);
    }

    /// Writes `message` of the type `type_name` at the level `Error` (`Error<T>(string?)`).
    fn error_with_type(&self, type_name: &'static str, message: &str) {
        self.write_with_type(message, type_name, LogLevel::Error);
    }

    /// Writes the message `args` at the level `Error` (`Error(string, params object[])`,
    /// `Error(IFormatProvider, string, params object[])`, `Error<T>(IFormatProvider, T)` and
    /// `Error<TArgument...>(IFormatProvider, string, TArgument...)`).
    fn error_format(&self, args: fmt::Arguments<'_>) {
        self.write(&args.to_string(), LogLevel::Error);
    }

    /// Writes the message `args` of the type `type_name` at the level `Error`
    /// (`Error<T>(string, params object[])`).
    fn error_format_with_type(&self, type_name: &'static str, args: fmt::Arguments<'_>) {
        self.write_with_type(&args.to_string(), type_name, LogLevel::Error);
    }

    /// Writes the message `function` returns at the level `Error`; the function is called only when
    /// the level is enabled (`Error(Func<string>)`).
    fn error_fn(&self, function: &dyn Fn() -> String) {
        if self.is_error_enabled() {
            self.write(&function(), LogLevel::Error);
        }
    }

    /// Writes the message `function` returns, of the type `type_name`, at the level `Error`; the
    /// function is called only when the level is enabled (`Error<T>(Func<string>)`).
    fn error_fn_with_type(&self, type_name: &'static str, function: &dyn Fn() -> String) {
        if self.is_error_enabled() {
            self.write_with_type(&function(), type_name, LogLevel::Error);
        }
    }

    /// Writes `exception` with the message `function` returns at the level `Error`; the function is
    /// called only when the level is enabled (`Error(Exception, Func<string>)`).
    fn error_exception_fn(&self, exception: &dyn Error, function: &dyn Fn() -> String) {
        if self.is_error_enabled() {
            self.write_exception(exception, &function(), LogLevel::Error);
        }
    }

    /// Writes `exception` with `message: exception` at the level `Error`
    /// (`ErrorException(string?, Exception)`).
    #[deprecated(note = "Use error_exception(exception, message)")]
    fn error_exception_message(&self, message: &str, exception: &dyn Error) {
        self.write_exception(exception, &format!("{message}: {exception}"), LogLevel::Error);
    }

    /// Writes `exception` with `message: exception`, the message returned by `function`, at the
    /// level `Error`; the function is called only when the level is enabled
    /// (`ErrorException(Func<string>, Exception)`).
    #[deprecated(note = "Use error_exception_fn(exception, function)")]
    fn error_exception_message_fn(&self, function: &dyn Fn() -> String, exception: &dyn Error) {
        if self.is_error_enabled() {
            self.write_exception(exception, &format!("{}: {exception}", function()), LogLevel::Error);
        }
    }

    /// Writes the text of `value` at the level `Fatal`, when the level is enabled (`Fatal<T>(T value)`).
    fn fatal_value(&self, value: &dyn fmt::Display) {
        if self.is_fatal_enabled() {
            self.write(&value.to_string(), LogLevel::Fatal);
        }
    }

    /// Writes `exception` with `message` at the level `Fatal` (`Fatal(Exception, string?)`).
    fn fatal_exception(&self, exception: &dyn Error, message: &str) {
        self.write_exception(exception, message, LogLevel::Fatal);
    }

    /// Writes `message` at the level `Fatal` (`Fatal(string?)`).
    fn fatal(&self, message: &str) {
        self.write(message, LogLevel::Fatal);
    }

    /// Writes `message` of the type `type_name` at the level `Fatal` (`Fatal<T>(string?)`).
    fn fatal_with_type(&self, type_name: &'static str, message: &str) {
        self.write_with_type(message, type_name, LogLevel::Fatal);
    }

    /// Writes the message `args` at the level `Fatal` (`Fatal(string, params object[])`,
    /// `Fatal(IFormatProvider, string, params object[])`, `Fatal<T>(IFormatProvider, T)` and
    /// `Fatal<TArgument...>(IFormatProvider, string, TArgument...)`).
    fn fatal_format(&self, args: fmt::Arguments<'_>) {
        self.write(&args.to_string(), LogLevel::Fatal);
    }

    /// Writes the message `args` of the type `type_name` at the level `Fatal`
    /// (`Fatal<T>(string, params object[])`).
    fn fatal_format_with_type(&self, type_name: &'static str, args: fmt::Arguments<'_>) {
        self.write_with_type(&args.to_string(), type_name, LogLevel::Fatal);
    }

    /// Writes the message `function` returns at the level `Fatal`; the function is called only when
    /// the level is enabled (`Fatal(Func<string>)`).
    fn fatal_fn(&self, function: &dyn Fn() -> String) {
        if self.is_fatal_enabled() {
            self.write(&function(), LogLevel::Fatal);
        }
    }

    /// Writes the message `function` returns, of the type `type_name`, at the level `Fatal`; the
    /// function is called only when the level is enabled (`Fatal<T>(Func<string>)`).
    fn fatal_fn_with_type(&self, type_name: &'static str, function: &dyn Fn() -> String) {
        if self.is_fatal_enabled() {
            self.write_with_type(&function(), type_name, LogLevel::Fatal);
        }
    }

    /// Writes `exception` with the message `function` returns at the level `Fatal`; the function is
    /// called only when the level is enabled (`Fatal(Exception, Func<string>)`).
    fn fatal_exception_fn(&self, exception: &dyn Error, function: &dyn Fn() -> String) {
        if self.is_fatal_enabled() {
            self.write_exception(exception, &function(), LogLevel::Fatal);
        }
    }

    /// Writes `exception` with `message: exception` at the level `Fatal`
    /// (`FatalException(string?, Exception)`).
    #[deprecated(note = "Use fatal_exception(exception, message)")]
    fn fatal_exception_message(&self, message: &str, exception: &dyn Error) {
        self.write_exception(exception, &format!("{message}: {exception}"), LogLevel::Fatal);
    }

    /// Writes `exception` with `message: exception`, the message returned by `function`, at the
    /// level `Fatal`; the function is called only when the level is enabled
    /// (`FatalException(Func<string>, Exception)`).
    #[deprecated(note = "Use fatal_exception_fn(exception, function)")]
    fn fatal_exception_message_fn(&self, function: &dyn Fn() -> String, exception: &dyn Error) {
        if self.is_fatal_enabled() {
            self.write_exception(exception, &format!("{}: {exception}", function()), LogLevel::Fatal);
        }
    }
}

/// The generic forms of [`IFullLogger`]: the type that writes is a type parameter
/// (`Debug<T>(string?)`, ...).
pub trait FullLoggerExt: IFullLogger {
    /// `Debug<T>(string?)`.
    fn debug_for<T: ?Sized>(&self, message: &str) {
        self.debug_with_type(std::any::type_name::<T>(), message);
    }

    /// `Debug<T>(string, params object[])`.
    fn debug_format_for<T: ?Sized>(&self, args: fmt::Arguments<'_>) {
        self.debug_format_with_type(std::any::type_name::<T>(), args);
    }

    /// `Debug<T>(Func<string>)`.
    fn debug_fn_for<T: ?Sized>(&self, function: impl Fn() -> String) {
        self.debug_fn_with_type(std::any::type_name::<T>(), &function);
    }

    /// `Info<T>(string?)`.
    fn info_for<T: ?Sized>(&self, message: &str) {
        self.info_with_type(std::any::type_name::<T>(), message);
    }

    /// `Info<T>(string, params object[])`.
    fn info_format_for<T: ?Sized>(&self, args: fmt::Arguments<'_>) {
        self.info_format_with_type(std::any::type_name::<T>(), args);
    }

    /// `Info<T>(Func<string>)`.
    fn info_fn_for<T: ?Sized>(&self, function: impl Fn() -> String) {
        self.info_fn_with_type(std::any::type_name::<T>(), &function);
    }

    /// `Warn<T>(string?)`.
    fn warn_for<T: ?Sized>(&self, message: &str) {
        self.warn_with_type(std::any::type_name::<T>(), message);
    }

    /// `Warn<T>(string, params object[])`.
    fn warn_format_for<T: ?Sized>(&self, args: fmt::Arguments<'_>) {
        self.warn_format_with_type(std::any::type_name::<T>(), args);
    }

    /// `Warn<T>(Func<string>)`.
    fn warn_fn_for<T: ?Sized>(&self, function: impl Fn() -> String) {
        self.warn_fn_with_type(std::any::type_name::<T>(), &function);
    }

    /// `Error<T>(string?)`.
    fn error_for<T: ?Sized>(&self, message: &str) {
        self.error_with_type(std::any::type_name::<T>(), message);
    }

    /// `Error<T>(string, params object[])`.
    fn error_format_for<T: ?Sized>(&self, args: fmt::Arguments<'_>) {
        self.error_format_with_type(std::any::type_name::<T>(), args);
    }

    /// `Error<T>(Func<string>)`.
    fn error_fn_for<T: ?Sized>(&self, function: impl Fn() -> String) {
        self.error_fn_with_type(std::any::type_name::<T>(), &function);
    }

    /// `Fatal<T>(string?)`.
    fn fatal_for<T: ?Sized>(&self, message: &str) {
        self.fatal_with_type(std::any::type_name::<T>(), message);
    }

    /// `Fatal<T>(string, params object[])`.
    fn fatal_format_for<T: ?Sized>(&self, args: fmt::Arguments<'_>) {
        self.fatal_format_with_type(std::any::type_name::<T>(), args);
    }

    /// `Fatal<T>(Func<string>)`.
    fn fatal_fn_for<T: ?Sized>(&self, function: impl Fn() -> String) {
        self.fatal_fn_with_type(std::any::type_name::<T>(), &function);
    }
}

impl<L: IFullLogger + ?Sized> FullLoggerExt for L {}
