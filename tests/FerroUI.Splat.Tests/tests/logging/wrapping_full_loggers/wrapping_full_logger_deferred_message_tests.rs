//! Ported from `Splat.Logging.Tests/WrappingFullLoggers/WrappingFullLoggerDeferredMessageTests.cs`
//! of Splat.
//! Adapted: `Debug(Exception, Func<string>)` is `debug_exception_fn`, and the deprecated
//! `DebugException(Func<string>, Exception)` is `debug_exception_message_fn` (the tests allow the
//! deprecation); the flag the factory sets is a `Cell<bool>`, and the `InvalidOperationException`
//! is a `TestException` with the same message. The tests that disable `Fatal` use a level above it
//! (`AboveFatal`), which `LogLevel` does not have, and are not ported. The tests of
//! `DebugException(Func<string>, Exception)` (`DebugException_Func_Enabled_Emits`, ...) are named
//! `debugexception_...`: the snake case of their names is that of the tests of `Debug(Exception,
//! Func<string>)` (`Debug_Exception_Func_Enabled_Emits`).

use super::wrapping_full_logger_test_factory::create_logger;
use crate::tests::common::{IMockLogTarget, TestException, TextLogger};
use ferroui_splat::{IFullLogger, LogLevel};
use std::cell::Cell;

const DEFERRED_MESSAGE: &str = "deferred-detail";

/// The exception of the tests (`new InvalidOperationException("wrapped-failure")`).
fn exception() -> TestException {
    TestException::with_message("wrapped-failure")
}

/// The deferred message; records that it was asked for (`Factory`).
fn factory(invoked: &Cell<bool>) -> String {
    invoked.set(true);
    DEFERRED_MESSAGE.to_owned()
}

/// The last entry has the deferred message at `expected_level`, which was asked for
/// (`AssertEmitted`).
fn assert_emitted(target: &TextLogger, expected_level: LogLevel, invoked: bool) {
    let logs = target.logs();
    let entry = logs.last().unwrap();
    assert!(entry.1.contains(DEFERRED_MESSAGE));
    assert_eq!(entry.0, expected_level);
    assert!(invoked);
}

/// Nothing was written, and the deferred message was not asked for (`AssertSuppressed`).
fn assert_suppressed(target: &TextLogger, invoked: bool) {
    assert!(target.logs().is_empty());
    assert!(!invoked);
}

#[test]
fn debug_exception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Debug);
    let invoked = Cell::new(false);

    logger.debug_exception_fn(&exception(), &|| factory(&invoked));

    assert_emitted(&target, LogLevel::Debug, invoked.get());
}

#[test]
fn debug_exception_func_disabled_does_not_emit() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.debug_exception_fn(&exception(), &|| factory(&invoked));

    assert_suppressed(&target, invoked.get());
}

#[test]
#[allow(deprecated)]
fn debugexception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Debug);
    let invoked = Cell::new(false);

    logger.debug_exception_message_fn(&|| factory(&invoked), &exception());

    assert_emitted(&target, LogLevel::Debug, invoked.get());
}

#[test]
#[allow(deprecated)]
fn debugexception_func_disabled_does_not_emit() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.debug_exception_message_fn(&|| factory(&invoked), &exception());

    assert_suppressed(&target, invoked.get());
}

#[test]
fn info_exception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Info);
    let invoked = Cell::new(false);

    logger.info_exception_fn(&exception(), &|| factory(&invoked));

    assert_emitted(&target, LogLevel::Info, invoked.get());
}

#[test]
fn info_exception_func_disabled_does_not_emit() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.info_exception_fn(&exception(), &|| factory(&invoked));

    assert_suppressed(&target, invoked.get());
}

#[test]
#[allow(deprecated)]
fn infoexception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Info);
    let invoked = Cell::new(false);

    logger.info_exception_message_fn(&|| factory(&invoked), &exception());

    assert_emitted(&target, LogLevel::Info, invoked.get());
}

#[test]
#[allow(deprecated)]
fn infoexception_func_disabled_does_not_emit() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.info_exception_message_fn(&|| factory(&invoked), &exception());

    assert_suppressed(&target, invoked.get());
}

#[test]
fn warn_exception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Warn);
    let invoked = Cell::new(false);

    logger.warn_exception_fn(&exception(), &|| factory(&invoked));

    assert_emitted(&target, LogLevel::Warn, invoked.get());
}

#[test]
fn warn_exception_func_disabled_does_not_emit() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.warn_exception_fn(&exception(), &|| factory(&invoked));

    assert_suppressed(&target, invoked.get());
}

#[test]
#[allow(deprecated)]
fn warnexception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Warn);
    let invoked = Cell::new(false);

    logger.warn_exception_message_fn(&|| factory(&invoked), &exception());

    assert_emitted(&target, LogLevel::Warn, invoked.get());
}

#[test]
#[allow(deprecated)]
fn warnexception_func_disabled_does_not_emit() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.warn_exception_message_fn(&|| factory(&invoked), &exception());

    assert_suppressed(&target, invoked.get());
}

#[test]
fn error_exception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Error);
    let invoked = Cell::new(false);

    logger.error_exception_fn(&exception(), &|| factory(&invoked));

    assert_emitted(&target, LogLevel::Error, invoked.get());
}

#[test]
fn error_exception_func_disabled_does_not_emit() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.error_exception_fn(&exception(), &|| factory(&invoked));

    assert_suppressed(&target, invoked.get());
}

#[test]
#[allow(deprecated)]
fn errorexception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Error);
    let invoked = Cell::new(false);

    logger.error_exception_message_fn(&|| factory(&invoked), &exception());

    assert_emitted(&target, LogLevel::Error, invoked.get());
}

#[test]
#[allow(deprecated)]
fn errorexception_func_disabled_does_not_emit() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.error_exception_message_fn(&|| factory(&invoked), &exception());

    assert_suppressed(&target, invoked.get());
}

#[test]
fn fatal_exception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.fatal_exception_fn(&exception(), &|| factory(&invoked));

    assert_emitted(&target, LogLevel::Fatal, invoked.get());
}

#[test]
#[allow(deprecated)]
fn fatalexception_func_enabled_emits() {
    let (logger, target) = create_logger(LogLevel::Fatal);
    let invoked = Cell::new(false);

    logger.fatal_exception_message_fn(&|| factory(&invoked), &exception());

    assert_emitted(&target, LogLevel::Fatal, invoked.get());
}

// Not ported: Fatal_Exception_Func_Disabled_Does_Not_Emit - an enum value that does not exist in Rust (AboveFatal, LogLevel.Fatal + 1)
// Not ported: FatalException_Func_Disabled_Does_Not_Emit - an enum value that does not exist in Rust (AboveFatal, LogLevel.Fatal + 1)
// Not ported: Fatal_Func_Disabled_Does_Not_Emit - an enum value that does not exist in Rust (AboveFatal, LogLevel.Fatal + 1)
// Not ported: Fatal_Generic_Func_Disabled_Does_Not_Emit - an enum value that does not exist in Rust (AboveFatal, LogLevel.Fatal + 1)
